use synd_auth::{
    device_flow::{provider, DeviceFlow},
    jwt,
};

pub struct DeviceFlows {
    pub github: DeviceFlow<provider::Github>,
    pub google: DeviceFlow<provider::Google>,
}

pub struct JwtService {
    pub google: jwt::google::JwtService,
}

impl JwtService {
    pub fn new() -> Self {
        Self {
            google: jwt::google::JwtService::default(),
        }
    }
}

pub struct Authenticator {
    pub device_flows: DeviceFlows,
    pub jwt_service: JwtService,
}

impl Authenticator {
    pub fn new() -> Self {
        Self {
            device_flows: DeviceFlows {
                github: DeviceFlow::new(provider::Github::default()),
                google: DeviceFlow::new(provider::Google::default()),
            },
            jwt_service: JwtService::new(),
        }
    }

    #[must_use]
    pub fn with_device_flows(self, device_flows: DeviceFlows) -> Self {
        Self {
            device_flows,
            ..self
        }
    }
}
