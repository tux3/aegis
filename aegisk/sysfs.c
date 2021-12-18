#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/kobject.h>
#include <linux/sysfs.h>
#include "monitor.h"
#include "sysfs.h"
#include "lock.h"

static struct kobject *aegisk_kobj;
static int lock_vt_flag;

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
	__ATTR(umh_pid, S_IRUSR, umh_pid_show, NULL);

static struct attribute *attrs[] = {
	&umh_pid_attribute.attr,
	&lock_vt_attribute.attr,
	NULL,
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
	aegisk_kobj = kobject_create_and_add("aegisk", NULL);
	if (!aegisk_kobj)
		return -ENOMEM;

	return sysfs_create_group(aegisk_kobj, &attr_group);
}
