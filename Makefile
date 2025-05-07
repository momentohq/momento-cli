.PHONY: all
## Generate sync unit tests, format, lint, and test
all: precommit

.PHONY: format
## Format all files
format:
	cargo fmt

.PHONY: lint
## Check the formatting of all files, run clippy on the source code, then run
## clippy on the tests (but allow expect to be used in tests)
lint:
	cargo fmt -- --check && \
	cargo clippy --all-features -- -D warnings -W clippy::unwrap_used && \
	cargo clippy --tests -- -D warnings -W clippy::unwrap_used

.PHONY: build
## Build project
build:
	cargo build --verbose

.PHONY: clean
## Remove build files
clean:
	cargo clean

.PHONY: clean-build
## Build project
clean-build: clean build

.PHONY: precommit
## Run clean-build and test as a step before committing.
precommit: clean-build lint test build-examples

.PHONY: test-unit
test-unit:
	cargo test --lib

.PHONY: test-sequentially
test-sequentially:
	./run_test_sequentially.sh

.PHONY: test
test: test-unit test-sequentially

.PHONY: run-cloud-linter
run-cloud-linter:
	AWS_PROFILE=dev cargo run -- preview cloud-linter --region us-west-2

# See <https://gist.github.com/klmr/575726c7e05d8780505a> for explanation.
.PHONY: help
help:
	@echo "$$(tput bold)Available rules:$$(tput sgr0)";echo;sed -ne"/^## /{h;s/.*//;:d" -e"H;n;s/^## //;td" -e"s/:.*//;G;s/\\n## /---/;s/\\n/ /g;p;}" ${MAKEFILE_LIST}|LC_ALL='C' sort -f|awk -F --- -v n=$$(tput cols) -v i=19 -v a="$$(tput setaf 6)" -v z="$$(tput sgr0)" '{printf"%s%*s%s ",a,-i,$$1,z;m=split($$2,w," ");l=n-i;for(j=1;j<=m;j++){l-=length(w[j])+1;if(l<= 0){l=n-i-length(w[j])-1;printf"\n%*s ",-i," ";}printf"%s ",w[j];}printf"\n";}'|more $(shell test $(shell uname) == Darwin && echo '-Xr')

