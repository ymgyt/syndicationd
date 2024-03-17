pub mod github {
    pub const CLIENT_ID: &str = "6652e5931c88e528a851";
}

pub mod google {
    pub const CLIENT_ID: &str = concat!(
        "387487893172-",
        "u28ebbv8lbl157jjeb7blsts8b5impio",
        ".apps.googleusercontent.com"
    );
    // This value is distributed as binary anyway, so it cannot be secret
    pub const CLIENT_ID2: &str = concat!("GOCSPX-", "igWWNOqW7hsV_", "08qdDx1P2s8YqlG");
}
