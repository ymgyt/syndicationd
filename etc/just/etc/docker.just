# Login ghcr
login:
    $env.GHCR_PAT | docker login ghcr.io -u USERNAME --password-stdin
