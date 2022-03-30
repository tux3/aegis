MAKEFILE_DIR:=$(dir $(realpath $(firstword $(MAKEFILE_LIST))))
KDIR ?= /usr/src/linux-headers-$(shell uname -r)/
MODULE = aegisk
MODULE_OBJ = $(MODULE).ko

.PHONY: clean rmmod insmod install quickInstall

all:
	$(MAKE) -C $(KDIR) M=$(MAKEFILE_DIR) modules

clean:
	$(MAKE) -C $(KDIR) M=$(MAKEFILE_DIR) clean

dkmsBuild: all
	test -f /var/lib/shim-signed/mok/MOK.priv && \
	  sudo /lib/modules/${kernelver}/source/scripts/sign-file sha256 \
	  /var/lib/shim-signed/mok/MOK.priv /var/lib/shim-signed/mok/MOK.der ${MODULE_OBJ} \
	  || true

rmmod:
	lsmod | grep -q "^$(MODULE)\s" && sudo rmmod $(MODULE) || true

insmod: all rmmod
	sudo insmod ./$(MODULE_OBJ)

install:
	$(MAKE) -C $(KDIR) M=$(MAKEFILE_DIR) modules_install

quickInstall:
	cp $(MODULE_OBJ) /lib/modules/`uname -r`/extra

dkmsInstall: all
	sudo mkdir -p /usr/src/aegisk-1.0/
	sudo cp Kbuild dkms.conf Makefile *.c *.h /usr/src/aegisk-1.0/
	sudo dkms remove -m aegisk -v 1.0 --all || true
	sudo dkms add -m aegisk -v 1.0
	sudo dkms autoinstall

