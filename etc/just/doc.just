import '.base.just'

# Build and open rustdoc
[no-cd]
open:
    cargo doc --open --no-deps
