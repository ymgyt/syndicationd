import '.base.just'

github_pat := env_var_or_default("GH_PAT", "")

# Update synd_api graphql schema
[no-cd]
schema:
    @graphql-client introspect-schema https://localhost:5959/graphql \
      --no-ssl \
      --header 'authorization: github {{ github_pat }}' \
      out> crates/synd_term/src/client/synd_api/schema.json

# Generate graphql code
[no-cd]
generate:
    @nu etc/scripts/graphql/generate.nu
