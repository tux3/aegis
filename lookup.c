#define pr_fmt(fmt) KBUILD_MODNAME ": " fmt

#include <linux/kprobes.h>

static DEFINE_MUTEX(kla_lookup_mutex);
unsigned long (*kla)(const char *name);

// Protected by kla_lookup_mutex
static int resolve_kla(void)
{
	struct kprobe kla_kp = { .symbol_name = "kallsyms_lookup_name" };

	int ret = register_kprobe(&kla_kp);
	if (ret < 0) {
		pr_err("resolve_kla: register_kprobe failed (%pe)\n",
		       ERR_PTR(ret));
		return ret;
	}
	pr_devel("resolved kallsyms_lookup_name");
	kla = (void *)kla_kp.addr;
	unregister_kprobe(&kla_kp);
	return 0;
}

unsigned long module_kallsyms_lookup_name(const char *name)
{
	mutex_lock(&kla_lookup_mutex);
	if (!kla && resolve_kla() < 0)
		return 0;
	mutex_unlock(&kla_lookup_mutex);
	pr_devel("resolving '%s' using kallsyms_lookup_name", name);
	return kla(name);
}
