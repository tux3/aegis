#include <linux/module.h>
#include <linux/types.h>
#include <linux/slab.h>

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

MODULE_LICENSE("GPL");
MODULE_AUTHOR("tux3 <barrdetwix@gmail.com>");
MODULE_DESCRIPTION("Aegis driver");
