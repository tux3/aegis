KDIR ?= /usr/src/linux-headers-$(shell uname -r)/
MODULE = aegisk
MODULE_OBJ = $(MODULE).ko

all:
	make -C $(KDIR) M=$(PWD) modules

clean:
	make -C $(KDIR) M=$(PWD) clean

insert: all
	lsmod | grep -q "^$(MODULE)\s" && sudo rmmod $(MODULE) || true
	sudo insmod ./$(MODULE_OBJ)

install:
	make -C $(KDIR) M=$(PWD) modules_install

quickInstall:
	cp $(MODULE_OBJ) /lib/modules/`uname -r`/extra
