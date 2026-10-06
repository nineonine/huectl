# Out-of-tree kernel module build. Run inside the VM, never on the host.
#
# How it works: `make -C $(KDIR) M=$(PWD)` runs the kernel's own build
# system (kbuild) from the headers dir, which reads `obj-m` below to learn
# which modules to build from this directory.

obj-m += hello.o

KDIR ?= /lib/modules/$(shell uname -r)/build

all:
	$(MAKE) -C $(KDIR) M=$(CURDIR) modules

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR) clean
