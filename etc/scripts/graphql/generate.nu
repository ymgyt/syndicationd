use std log

# Generate graphql client code from query
def main [ ] {

  let synd_term =  {
      schema: "crates/synd_term/src/client/synd_api/schema.json",
      out: "crates/synd_term/src/client/synd_api/generated",
      var_derives: "Debug,Clone,PartialEq,Eq",
      res_derives: "Debug,Clone,PartialEq,Eq",
      scalar: "crate::client::synd_api::scalar",
  }
  let synd_api = {
    schema: "crates/synd_api/src/client/github/schema.json",
    out: "crates/synd_api/src/client/github/generated",
    var_derives: "Debug",
    res_derives: "Debug",
    scalar: null,
  }

  let queries = [
    ($synd_term | insert gql { "crates/synd_term/src/client/synd_api/query.gql"} ),
    ($synd_term | insert gql { "crates/synd_term/src/client/synd_api/mutation.gql"} ),
    ($synd_api  | insert gql { "crates/synd_api/src/client/github/query.gql"} ),
  ]

  $queries | each {|x| 

    let scalars = if ($x.scalar != null) {
      ["--custom-scalars-module"  $x.scalar]
    } else {
      []
    }

    log info $"Generate rust code from ($x.gql) to ($x.out)"

    (
      graphql-client generate
        --schema-path $x.schema
        --output-directory $x.out
        --variables-derives $x.var_derives
        --response-derives $x.res_derives
        ...$scalars
        $x.gql
    )
  }

  return
}
