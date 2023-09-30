#[cfg(test)]
mod tests {
    #[test]
    #[allow(unused_variables)]
    fn merge() {
        let input = [
            "Exception in thread \"main\" java.lang.IllegalStateException: ..null property\n",
            "     at com.example.myproject.Author.getBookIds(xx.java:38)\n",
            "     at com.example.myproject.Bootstrap.main(Bootstrap.java:14)\n",
            "Caused by: java.lang.NullPointerException\n",
            "     at com.example.myproject.Book.getId(Book.java:22)\n",
            "     at com.example.myproject.Author.getBookIds(Author.java:35)\n",
            "     ... 1 more\n",
            "single line\n",
        ];

        let want = [
            "Exception in thread \"main\" java.lang.IllegalStateException: ..null property\n\
     at com.example.myproject.Author.getBookIds(xx.java:38)\n\
     at com.example.myproject.Bootstrap.main(Bootstrap.java:14)\n\
Caused by: java.lang.NullPointerException\n\
     at com.example.myproject.Book.getId(Book.java:22)\n\
     at com.example.myproject.Author.getBookIds(Author.java:35)\n\
     ... 1 more\n",
            "single line\n",
        ];
    }
}
