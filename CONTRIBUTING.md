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

```
cargo test
```

### Deploying

After merge a pr will be created in this repo https://github.com/momentohq/homebrew-tap. Once the pr passes all checks, approve the pr and label is as `pr-pull`. It will then get automatically merged by the homebrew bot, and a release will be created for it.
