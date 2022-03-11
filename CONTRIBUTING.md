### Starting Off

```
git submodule init
git submodule sync
git submodule update --recursive --remote
```

### Building

```
cargo build
```

### Testing

Make sure you have `~/.momento/credentials` and `~/.momento/config` files with the following data.

`~/.momento/credentials`

```
[default]
token=<YOUR_TOKEN>
[YOUR_TEST_PROFILE]
token=<YOUR_TOKEN>
```

`~/.momento/config`

```
[default]
cache=<YOUR_TEST_CACHE_DEFAULT>
ttl=600
[YOUR_TEST_PROFILE]
cache=<YOUR_TEST_CACHE_WITH_PROFILE>
ttl=700
```

```
export TEST_CACHE_DEFAULT=<YOUR_TEST_CACHE_DEFAULT>
export TEST_CACHE_WITH_PROFILE=<YOUR_TEST_CACHE_WITH_PROFILE>
export TEST_PROFILE=<YOUR_TEST_PROFILE>
./run_test_sequentially.sh
cargo clippy --all-targets --all-features -- -D warnings
```

### Deploying

After merge a pr will be created in this repo https://github.com/momentohq/homebrew-tap. Once the pr passes all checks, approve the pr and label is as `pr-pull`. It will then get automatically merged by the homebrew bot, and a release will be created for it.
