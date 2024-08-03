mod docker 'docker.just'
mod codecov 'codecov.just'

# Reinstall synd-term
[no-cd]
reinstall:
    @nu etc/scripts/reinstall.nu
