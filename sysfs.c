#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/kobject.h>
#include <linux/sysfs.h>
#include "monitor.h"
#include "sysfs.h"

struct kobject *aegisk_kobj;

static ssize_t umh_pid_show(struct kobject *kobj, struct kobj_attribute *attr,
		      char *buf)
{
	return sprintf(buf, "%d\n", aegisc_umh_get_pid());
}

static struct kobj_attribute umh_pid_attribute =
	__ATTR(umh_pid, S_IRUSR, umh_pid_show, NULL);

static struct attribute *attrs[] = {
	&umh_pid_attribute.attr,
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
