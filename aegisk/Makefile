MAKEFILE_DIR:=$(dir $(realpath $(firstword $(MAKEFILE_LIST))))
KDIR ?= /usr/src/linux-headers-$(shell uname -r)/
MODULE = aegisk
MODULE_OBJ = $(MODULE).ko

.PHONY: clean rmmod insmod install quickInstall

all:
	make -C $(KDIR) M=$(MAKEFILE_DIR) modules

clean:
	make -C $(KDIR) M=$(MAKEFILE_DIR) clean

rmmod:
	lsmod | grep -q "^$(MODULE)\s" && sudo rmmod $(MODULE) || true

insmod: all rmmod
	sudo insmod ./$(MODULE_OBJ)

install:
	make -C $(KDIR) M=$(MAKEFILE_DIR) modules_install

quickInstall:
	cp $(MODULE_OBJ) /lib/modules/`uname -r`/extra

dkmsInstall:
	sudo mkdir /usr/src/aegisk-1.0/
	sudo cp Kbuild dkms.conf Makefile *.c *.h /usr/src/aegisk-1.0/
	dkms add -m aegisk -v 1.0

