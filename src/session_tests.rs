#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html() {
        // Test bold formatting
        assert_eq!(
            markdown_to_html("This is **bold** text"),
            "This is <b>bold</b> text"
        );

        // Test code formatting
        assert_eq!(
            markdown_to_html("Use `!alias` command"),
            "Use <tt>!alias</tt> command"
        );

        // Test combined formatting
        assert_eq!(
            markdown_to_html("**Bold** and `code` together"),
            "<b>Bold</b> and <tt>code</tt> together"
        );

        // Test multiple bold sections
        assert_eq!(
            markdown_to_html("**First** and **Second** bold"),
            "<b>First</b> and <b>Second</b> bold"
        );

        // Test with HTML entities that need escaping in bold text
        assert_eq!(
            markdown_to_html("**<script>** is dangerous"),
            "<b>&lt;script&gt;</b> is dangerous"
        );

        // Test unclosed code block
        assert_eq!(
            markdown_to_html("Start `code here"),
            "Start <tt>code here</tt>"
        );

        // Test bullets (should be preserved)
        assert_eq!(
            markdown_to_html("• First item\n• Second item"),
            "• First item\n• Second item"
        );
    }
}
