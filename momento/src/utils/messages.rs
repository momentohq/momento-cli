pub fn failed_to_get_profile(profile: &str) -> std::string::String {
    format!("failed to get config for profile {profile}, please run 'momento configure' to configure your profile")
}
