### Starting Off
1. git submodule init
1. git submodule sync
1. git submodule update --recursive --remote

### Building
1. cargo build

### Testing
1. cargo test

### Deploying
After merge a pr will be created in this repo https://github.com/momentohq/homebrew-tap. Once the pr passes all checks, approve the pr and label is as `pr-pull`. It will then get automatically merged by the homebrew bot, and a release will be created for it.
