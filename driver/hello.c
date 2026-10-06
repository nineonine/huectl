// SPDX-License-Identifier: GPL-2.0 OR MIT
/*
 * Phase 0 / Step 5: the smallest possible kernel module.
 *
 * Proves the VM toolchain works: headers match the running kernel,
 * the module builds, loads, logs, and unloads cleanly.
 */
#include <linux/init.h>
#include <linux/module.h>

static int __init hello_init(void)
{
	pr_info("huectl: hello, kernel\n");
	return 0;	/* nonzero (a negative errno) would make insmod fail */
}

static void __exit hello_exit(void)
{
	pr_info("huectl: goodbye, kernel\n");
}

module_init(hello_init);
module_exit(hello_exit);

/* Must be GPL-compatible, or the kernel hides GPL-only symbols and taints. */
MODULE_LICENSE("Dual MIT/GPL");
MODULE_DESCRIPTION("huectl hello-world module");
