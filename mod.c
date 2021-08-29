#include <linux/module.h>
#include <linux/types.h>
#include <linux/slab.h>
#include <asm/cpufeatures.h>
#include <asm/cpu_device_id.h>

static void __cleanup(void)
{
}

static void __exit cleanup(void)
{
	__cleanup();
}

static int __init init(void)
{
	int err;

	return 0;

fail:
	__cleanup();
	return err;
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
MODULE_DESCRIPTION("Aegis driver");
