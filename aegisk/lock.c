#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/console.h>
#include <linux/errno.h>
#include <linux/tty.h>
#include <linux/vt_kern.h>
#include "lookup.h"
#include "lock.h"

static const int LOCKED_VT_NUM = 25; // Arbitrary. Hope you don't use this VT!
static int previous_vt;
static int (*_vt_ioctl)(struct tty_struct *tty, unsigned int cmd,
			unsigned long arg);

int aegisk_lock_vt(void)
{
	struct tty_struct *tty = get_current_tty();
	if (!tty)
		return -ENOTTY;

	console_lock();
	previous_vt = fg_console;
	console_unlock();

	_vt_ioctl(tty, VT_ACTIVATE, LOCKED_VT_NUM + 1);
	_vt_ioctl(tty, VT_WAITACTIVE, LOCKED_VT_NUM + 1);
	_vt_ioctl(tty, VT_LOCKSWITCH, 0);

	tty_kref_put(tty);
	return 0;
}

int aegisk_unlock_vt(void)
{
	struct tty_struct *tty = get_current_tty();
	if (!tty)
		return -ENOTTY;

	_vt_ioctl(tty, VT_UNLOCKSWITCH, 0);
	_vt_ioctl(tty, VT_ACTIVATE, previous_vt + 1);
	tty_kref_put(tty);
	return 0;
}

int init_locking(void)
{
	_vt_ioctl = (void *)module_kallsyms_lookup_name("vt_ioctl");
	if (!_vt_ioctl)
		return -ENOENT;
	return 0;
}
