#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/completion.h>
#include <linux/delay.h>
#include <linux/file.h>
#include <linux/fs.h>
#include <linux/kthread.h>
#include <linux/mount.h>
#include <linux/mm.h>
#include <linux/sched/signal.h>
#include <linux/sched/task.h>
#include <linux/suspend.h>
#include <linux/types.h>
#include <linux/umh.h>
#include "monitor.h"

static const char *aegisc_path = "/usr/local/sbin/aegisc";

static DECLARE_COMPLETION(aegisc_umh_startup_done);
static int aegisc_umh_statup_err;

static DEFINE_MUTEX(aegisc_task_mutex);
static struct task_struct *aegisc_task;
static int aegisc_task_disable;

static struct task_struct *aegisc_monitor_task;

// This only does some very basic checks
// It is vulnerable to TOCTOU, but that's OK (root can do worse than TOCTOU)
static int sanity_check_aegisc_file(void)
{
	int ret = 0;
	struct file *file = filp_open(aegisc_path, O_LARGEFILE | O_RDONLY, 0);
	if (IS_ERR(file)) {
		pr_err("Failed to open usermode helper file: %pe\n", file);
		// insmod would interpret ENOENT as "Unknown symbol in module"
		if (PTR_ERR(file) == -ENOENT)
			return -ENOPKG;
		return PTR_ERR(file);
	}

	if (!S_ISREG(file_inode(file)->i_mode) ||
	    (file->f_path.mnt->mnt_flags & MNT_NOEXEC) ||
	    (file->f_path.mnt->mnt_sb->s_iflags & SB_I_NOEXEC)) {
		ret = -EACCES;
		goto out;
	}

	if (inode_is_open_for_write(file_inode(file)))
		ret = -ETXTBSY;

out:
	fput(file);
	return ret;
}

static int aegisc_umh_init(struct subprocess_info *info, struct cred *new)
{
	mutex_lock(&aegisc_task_mutex);
	WARN_ON(aegisc_task);
	if (aegisc_task_disable) {
		mutex_unlock(&aegisc_task_mutex);
		return -EBUSY;
	}

	aegisc_task = get_task_struct(current);
	pr_info("Usermode helper init (pid %u)\n", task_pid_nr(aegisc_task));
	mutex_unlock(&aegisc_task_mutex);

	complete_all(&aegisc_umh_startup_done);
	return 0;
}

static void aegisc_umh_cleanup(struct subprocess_info *info)
{
	mutex_lock(&aegisc_task_mutex);
	WARN_ON(!aegisc_task);
	put_task_struct(aegisc_task);
	aegisc_task = NULL;
	mutex_unlock(&aegisc_task_mutex);
}

static int aegisc_umh_disable_and_kill(void)
{
	int ret = 0;

	mutex_lock(&aegisc_task_mutex);
	aegisc_task_disable = 1;
	if (!aegisc_task)
		goto dead;
	ret = send_sig_info(SIGKILL, SEND_SIG_PRIV, aegisc_task);

dead:
	mutex_unlock(&aegisc_task_mutex);
	return ret;
}

static int run_aegisc_umh(void)
{
	struct subprocess_info *aegisc_sub_info;
	char *argv[2], *envp[1];
	argv[0] = (char *)aegisc_path;
	argv[1] = NULL;
	envp[0] = NULL;

	aegisc_sub_info = call_usermodehelper_setup(aegisc_path, argv, envp,
						    GFP_KERNEL, aegisc_umh_init,
						    aegisc_umh_cleanup, NULL);
	if (!aegisc_sub_info)
		return -ENOMEM;

	return call_usermodehelper_exec(aegisc_sub_info, UMH_WAIT_PROC | UMH_KILLABLE);
}

// kthread_stop() doesn't signal, but it should still interrupt us
static unsigned long ssleep_lightly(unsigned int secs)
{
	unsigned long timeout = msecs_to_jiffies(secs * 1000) + 1;

	while (timeout && !signal_pending(current) && !kthread_should_stop())
		timeout = schedule_timeout_interruptible(timeout);
	return jiffies_to_msecs(timeout);
}

static int aegisc_monitor_thread(void *unused)
{
	int ret;

	while (!kthread_should_stop()) {
		ret = run_aegisc_umh();
		pr_warn("Usermode helper has quit (%pe)\n", ERR_PTR(ret));
		if (ret) {
			// Failing on first run means we fail module init
			if (!completion_done(&aegisc_umh_startup_done)) {
				WRITE_ONCE(aegisc_umh_statup_err, ret);
				complete(&aegisc_umh_startup_done);
				return ret;
			}
		}

		if (kthread_should_stop())
			break;

		// Don't frantically restart, in case of crash-loops
		ssleep_lightly(5);
	}

	ret = aegisc_umh_disable_and_kill();
	if (ret < 0) {
		pr_err("Failed to kill aegisc usermode helper from inside monitor thread: %pe\n",
		       ERR_PTR(ret));
		return ret;
	}

	return 0;
}

pid_t aegisc_umh_get_pid(void)
{
	pid_t pid = 0;
	mutex_lock(&aegisc_task_mutex);
	if (aegisc_task)
		pid = task_pid_nr(aegisc_task);
	mutex_unlock(&aegisc_task_mutex);
	return pid;
}

static void stop_aegisc_monitor_thread(void)
{
	int err = aegisc_umh_disable_and_kill();
	if (err)
		pr_err("Failed to kill aegisc usermode helper while stopping monitor thread: %pe\n",
		       ERR_PTR(err));

	if (aegisc_monitor_task)
		kthread_stop(aegisc_monitor_task); // Not interruptible!
	aegisc_monitor_task = NULL;
}

static int start_aegisc_monitor_thread(void)
{
	int ret;

	if (WARN_ON(aegisc_monitor_task))
		return -EBUSY;
	if ((ret = sanity_check_aegisc_file()))
		return ret;

	aegisc_task_disable = 0;
	aegisc_monitor_task =
		kthread_create(aegisc_monitor_thread, NULL, "aegisc_monitor");
	if (IS_ERR(aegisc_monitor_task)) {
		ret = PTR_ERR(aegisc_monitor_task);
		aegisc_monitor_task = NULL;
	} else {
		wake_up_process(aegisc_monitor_task);
	}
	return ret;
}

#ifdef CONFIG_PM_SLEEP
static int aegisk_pm_notification(struct notifier_block *nb, unsigned long action,
			      void *data)
{
	int ret;

	if (action == PM_HIBERNATION_PREPARE || action == PM_SUSPEND_PREPARE) {
		pr_info("pre suspend notification, stopping umh and monitor thread");
		stop_aegisc_monitor_thread();
		return NOTIFY_OK;
	} else if (action == PM_POST_HIBERNATION || action == PM_POST_SUSPEND) {
		pr_info("post suspend notification, restarting umh and monitor thread");
		ret = start_aegisc_monitor_thread();
		if (ret) {
			pr_err("Failed to restart monitor thread (%pe)", ERR_PTR(ret));
			return 0;
		}
		return NOTIFY_OK;
	} else {
		return 0;
	}
}

static struct notifier_block pm_notifier = { .notifier_call = aegisk_pm_notification };
#endif

void aegisc_monitor_cleanup(void)
{
#ifdef CONFIG_PM_SLEEP
	unregister_pm_notifier(&pm_notifier);
#endif

	stop_aegisc_monitor_thread();
}

int aegisc_monitor_init(void)
{
	int ret = start_aegisc_monitor_thread();
	if (ret)
		return ret;

	pr_debug("Monitor thread started, waiting for first aegisc exec\n");
	if (wait_for_completion_interruptible(&aegisc_umh_startup_done)) {
		pr_warn("Interrupted while waiting for first aegisc exec\n");
		stop_aegisc_monitor_thread();
		return -EINTR;
	}

#ifdef CONFIG_PM_SLEEP
	ret = register_pm_notifier(&pm_notifier);
	if (ret) {
		pr_err("Failed to register pm notifier for umh monitor\n");
		stop_aegisc_monitor_thread();
		return ret;
	}
#endif

	pr_debug("Monitor thread and usermode helper started successfully\n");
	return READ_ONCE(aegisc_umh_statup_err);
}
