import '.base.just'

alias unused := machete

# Check unused dependencies
[no-cd]
machete:
    cargo machete
