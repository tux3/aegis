#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <asm/cpufeatures.h>
#include <asm/cpu_device_id.h>
#include <linux/module.h>
#include "monitor.h"

static void __cleanup(void)
{
	pr_debug("Starting module cleanup\n");
	stop_aegisc_monitor_thread();
	pr_debug("Finished module cleanup\n");
}

static void __exit cleanup(void)
{
	__cleanup();
}

static int __init init(void)
{
	int ret;

	ret = start_aegisc_monitor_thread();
	if (ret)
		return ret;

	return 0;
}

module_init(init);
module_exit(cleanup);

static const struct x86_cpu_id any_cpu_id[] = {
	X86_MATCH_FEATURE(X86_FEATURE_ANY, NULL),
	{}
};
MODULE_DEVICE_TABLE(x86cpu, any_cpu_id);

MODULE_LICENSE("GPL");
MODULE_AUTHOR("tux3 <barrdetwix@gmail.com>");
MODULE_DESCRIPTION("Aegis kernel driver");
