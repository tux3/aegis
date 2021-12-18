.PHONY: clean debug release

all: debug

debug:
	cd aegisk && make
	cargo build --all
	cd app && ./gradlew assembleDebug

release:
	cd aegisk && make
	cargo build --release --all
	cd app && ./gradlew assembleRelease

clean:
	cd aegisk && make clean
	cargo clean
	cd app && ./gradlew clean

