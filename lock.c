#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/errno.h>
#include "lock.h"

int aegisk_lock_vt(void)
{
	return -ENOSYS;
}

int aegisk_unlock_vt(void)
{
	return -ENOSYS;
}
