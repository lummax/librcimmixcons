CARGO = cargo
CARGO_OPTS =
PKGVER = 0

all: build

test:
	$(CARGO) test $(CARGO_OPTS)
	./tests/run_integration_tests.sh 

check:
	$(MAKE) test
	$(MAKE) test CARGO_OPTS="$(CARGO_OPTS) --features valgrind"

install:
	mkdir -p "${PREFIX}/lib/"
	install -m 0755 \
		target/release/librcimmixcons-*.so \
		"${PREFIX}/lib/librcimmixcons.so.${PKGVER}"
	ldconfig -n "${PREFIX}/lib/"
	cd "${PREFIX}/lib"
	ln -s "librcimmixcons.so.${PKGVER}" librcimmixcons.so

%:
	$(CARGO) $* $(CARGO_OPTS)

.PHONY: all test check install
