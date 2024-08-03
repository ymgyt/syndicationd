import '.base.just'

github_pat := env_var_or_default("GH_PAT", "")

# Update synd_api graphql schema
[no-cd]
schema:
    @graphql-client introspect-schema https://localhost:5959/graphql --no-ssl \
      --header 'authorization: github {{ github_pat }}' out> crates/synd_term/gql/schema.json

# Generate graphql code
[no-cd]
generate:
    graphql-client generate \
      --schema-path crates/synd_term/gql/schema.json \
      --output-directory crates/synd_term/src/client/generated \
      --variables-derives "Debug,Clone,PartialEq,Eq" \
      --response-derives "Debug,Clone,PartialEq,Eq" \
      --custom-scalars-module "crate::client::scalar" \
      crates/synd_term/gql/query.gql

    graphql-client generate \
      --schema-path crates/synd_term/gql/schema.json \
      --output-directory crates/synd_term/src/client/generated \
      --variables-derives "Debug,Clone,PartialEq,Eq" \
      --response-derives "Debug,Clone,PartialEq,Eq" \
      --custom-scalars-module "crate::client::scalar" \
      crates/synd_term/gql/mutation.gql

    graphql-client generate \
      --schema-path crates/synd_api/src/client/github/schema.json \
      --output-directory crates/synd_api/src/client/github \
      --variables-derives "Debug" \
      --response-derives "Debug" \
      crates/synd_api/src/client/github/query.gql
