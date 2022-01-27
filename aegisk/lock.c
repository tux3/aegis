#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/console.h>
#include <linux/errno.h>
#include <linux/tty.h>
#include <linux/vt_kern.h>
#include "lookup.h"
#include "lock.h"

static const int LOCKED_VT_NUM = 25; // Arbitrary. Hope you don't use this VT!
static int previous_vt;

static int (*_vt_waitactive)(int nr);
static int (*_vc_allocate)(unsigned int curcons);
static int (*_set_console)(int nr);
static bool* _vt_dont_switch;

static int aegis_vt_activate(int vt_num)
{
	int ret;

	if (vt_num == 0 || vt_num > MAX_NR_CONSOLES)
		return -ENXIO;
	vt_num--;

	console_lock();
	ret = _vc_allocate(vt_num);
	console_unlock();
	if (ret)
		return ret;
	_set_console(vt_num);
	return 0;
}

static int aegis_vt_wait_active(int vt_num)
{
	if (vt_num == 0 || vt_num > MAX_NR_CONSOLES)
		return -ENXIO;
	return _vt_waitactive(vt_num);
}

static int aegis_vt_lockswitch(bool lock)
{
	*_vt_dont_switch = lock;
	return 0;
}

int aegisk_lock_vt(void)
{
	int ret;

	console_lock();
	previous_vt = fg_console;
	console_unlock();

	if ((ret = aegis_vt_activate(LOCKED_VT_NUM + 1)))
		return ret;
	aegis_vt_wait_active(LOCKED_VT_NUM + 1);
	aegis_vt_lockswitch(true);

	return 0;
}

int aegisk_unlock_vt(void)
{
	aegis_vt_lockswitch(false);
	aegis_vt_activate(previous_vt + 1);
	return 0;
}

int init_locking(void)
{
	_vt_dont_switch = (void *)module_kallsyms_lookup_name("vt_dont_switch");
	if (!_vt_dont_switch)
		return -ENOENT;
	_vt_waitactive = (void *)module_kallsyms_lookup_name("vt_waitactive");
	if (!_vt_waitactive)
		return -ENOENT;
	_vc_allocate = (void *)module_kallsyms_lookup_name("vc_allocate");
	if (!_vc_allocate)
		return -ENOENT;
	_set_console = (void *)module_kallsyms_lookup_name("set_console");
	if (!_set_console)
		return -ENOENT;
	return 0;
}
