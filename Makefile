CARGO = cargo
CARGO_OPTS =
PREFIX = /usr
PKGVER = 0

all: build

test:
	$(CARGO) test $(CARGO_OPTS)
	./tests/run_integration_tests.sh

check:
	$(MAKE) test
	$(MAKE) test CARGO_OPTS="$(CARGO_OPTS) --features valgrind"

install:
	mkdir -p "${PREFIX}/include/"
	install -m 0644 \
		src/rcimmixcons.h \
		"${PREFIX}/include/rcimmixcons.h"
	mkdir -p "${PREFIX}/lib/"
	install -m 0755 \
		target/release/librcimmixcons.so \
		"${PREFIX}/lib/librcimmixcons.so.${PKGVER}"
	ldconfig -n "${PREFIX}/lib/"
	ln -s "librcimmixcons.so.${PKGVER}" "${PREFIX}/lib/librcimmixcons.so"

check-install:
	USE_GLOBAL_INSTALL=y ./tests/run_integration_tests.sh

%:
	$(CARGO) $* $(CARGO_OPTS)

.PHONY: all test check install
