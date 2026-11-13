/// Interpret the typed boolean result from gdbus Properties.Get.
pub fn host_registered(reply: &str) -> bool {
    reply
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        == "(<true>,)"
}
