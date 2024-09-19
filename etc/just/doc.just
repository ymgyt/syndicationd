import '.base.just'

# Build and open rustdoc
[no-cd]
open:
    cargo doc --open

# Open document with private items
[no-cd]
private:
    cargo doc --open --document-private-items --no-deps
