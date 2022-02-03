#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/ktime.h>
#include <linux/kobject.h>
#include <linux/sysfs.h>
#include <linux/sched/task.h>
#include <linux/timekeeping.h>
#include <linux/suspend.h>
#include <linux/reboot.h>
#include "monitor.h"
#include "sysfs.h"
#include "lock.h"

enum { AEGISK_POWEROFF = 0, AEGISK_REBOOT };

static struct task_struct *aegisk_pm_task;
static struct kobject *aegisk_kobj;
static int lock_vt_flag;
static u64 insert_time_utc_ns;
static u64 boot_time_ns;

static int aegisk_do_pm_task(void *pm_action)
{
	if (pm_action == (void*)AEGISK_POWEROFF)
		kernel_power_off();
	else if (pm_action == (void*)AEGISK_REBOOT)
		kernel_restart(NULL);
	return -EINVAL;
}

static ssize_t lock_vt_show(struct kobject *kobj, struct kobj_attribute *attr,
			    char *buf)
{
	return sprintf(buf, "%d\n", lock_vt_flag);
}

static ssize_t lock_vt_store(struct kobject *kobj, struct kobj_attribute *attr,
			     const char *buf, size_t count)
{
	int new;
	int ret = kstrtoint(buf, 0, &new);
	if (ret < 0)
		return ret;

	if (new == lock_vt_flag)
		return count;
	ret = new ? aegisk_lock_vt() : aegisk_unlock_vt();
	if (ret < 0) {
		pr_err("Failed to set lock_vt to %d: %pe\n", new, ERR_PTR(ret));
		return ret;
	}
	pr_info("Set lock_vt to %d\n", new);
	lock_vt_flag = new;
	return count;
}

static struct kobj_attribute lock_vt_attribute =
	__ATTR(lock_vt, S_IRUSR | S_IWUSR, lock_vt_show, lock_vt_store);

static ssize_t umh_pid_show(struct kobject *kobj, struct kobj_attribute *attr,
			    char *buf)
{
	return sprintf(buf, "%d\n", aegisc_umh_get_pid());
}

static struct kobj_attribute umh_pid_attribute =
	__ATTR(umh_pid, S_IRUSR | S_IRGRP | S_IROTH, umh_pid_show, NULL);

static ssize_t alert_store(struct kobject *kobj, struct kobj_attribute *attr,
			   const char *buf, size_t count)
{
	if (task_tgid_nr(current) != aegisc_umh_get_pid())
		return -EPERM;
	if (count > 1024)
		return -E2BIG;
	pr_emerg("Alert from usermode helper: %s\n", buf);
	return count;
}

static struct kobj_attribute alert_attribute =
	__ATTR(alert, S_IWUSR, NULL, alert_store);

static ssize_t power_store(struct kobject *kobj, struct kobj_attribute *attr,
			   const char *buf, size_t count)
{
	void* pm_action;
	if (task_tgid_nr(current) != aegisc_umh_get_pid())
		return -EPERM;
	if (aegisk_pm_task)
		return -EBUSY;

	if (strncmp(buf, "poweroff", count) == 0) {
		pm_action = (void*)AEGISK_POWEROFF;
	} else if (strncmp(buf, "reboot", count) == 0) {
		pm_action = (void*)AEGISK_REBOOT;
	} else {
		return -EINVAL;
	}
	ksys_sync_helper();
	aegisk_pm_task = kthread_create(aegisk_do_pm_task, pm_action, "aegisk_pm");
	if (IS_ERR(aegisk_pm_task)) {
		return PTR_ERR(aegisk_pm_task);
	}
	wake_up_process(aegisk_pm_task);
	// We don't need to clean this up, we're going to reboot/poweroff...

	return count;
}

static struct kobj_attribute power_attribute =
	__ATTR(power, S_IWUSR, NULL, power_store);

static ssize_t insert_time_show(struct kobject *kobj,
				struct kobj_attribute *attr, char *buf)
{
	return sprintf(buf, "%llu %llu\n", insert_time_utc_ns, boot_time_ns);
}

static struct kobj_attribute insert_time_attribute = __ATTR(
	insert_time, S_IRUSR | S_IRGRP | S_IROTH, insert_time_show, NULL);

static struct attribute *attrs[] = {
	&umh_pid_attribute.attr, &lock_vt_attribute.attr,
	&alert_attribute.attr,	 &insert_time_attribute.attr,
	&power_attribute.attr,	 NULL,
};

static struct attribute_group attr_group = {
	.attrs = attrs,
};

void aegisk_cleanup_sysfs(void)
{
	if (!aegisk_kobj)
		return;
	sysfs_remove_group(aegisk_kobj, &attr_group);
	kobject_del(aegisk_kobj);
}

int aegisk_init_sysfs(void)
{
	insert_time_utc_ns = ktime_get_real_ns();
	boot_time_ns = ktime_get_boottime_ns();

	aegisk_kobj = kobject_create_and_add("aegisk", NULL);
	if (!aegisk_kobj)
		return -ENOMEM;

	return sysfs_create_group(aegisk_kobj, &attr_group);
}
