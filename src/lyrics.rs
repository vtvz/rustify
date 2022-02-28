mod genius;
mod musixmatch;

trait SearchResult {
    fn lyrics(&self) -> &Vec<String>;
    fn tg_link(&self, text: &str) -> String;
}
