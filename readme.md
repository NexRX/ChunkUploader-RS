# Info

A simple command-line tool for uploading data to a given endpoint in chunks via a Content-Range.

##### Help

```
Chunk Uploader - Help
         -f, --file    File to upload
         -c, --chunk   Chunk size to use for upload
         -u, --url     URL to upload to
         -r, --range   Byte range of the file to upload e.g. 0-1000 for first 1000 bytes (Default: Input file's byte range [0-filesize])
         -m, --method  HTTP Method to use (Default: PUT)
         -h, --help    Show help (This command)
         -v, --version Show version
```
