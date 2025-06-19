# MDN Content Zed extension

## Local setup

```
git clone https://github.com/fiji-flo/mdn-content-zed.git
```

In Zed go to `install dev extension`.

In the content checkout: create a file `.zed/settings.json` with the following content.

```json
{
  "file_types": {
    "Markdown MDN": ["**/files/en-us/**/*.md"]
  }
}
```
