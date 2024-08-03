import '../.base.just'

# Validate codecov.yaml
[no-cd]
validate:
    open codecov.yaml | to yaml | http post https://codecov.io/validate $in

