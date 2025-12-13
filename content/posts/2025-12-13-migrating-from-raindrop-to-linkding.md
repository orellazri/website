+++
title = "Migrating My Bookmarks From Raindrop to Linkding"
date = "2025-12-13"

[taxonomies]
tags=["self-hosting", "ai"]
+++

I recently decided to migrate my bookmarks from Raindrop.io to [Linkding](https://github.com/sissbruecker/linkding), a self-hosted bookmark manager. I rather own my bookmarks' data instead of using a third-party service for this simple use-case.

While Linkding has a built-in HTML import feature, it doesn't preserve the folder structure from Raindrop so all bookmarks end up unorganized. It also doesn't help that Raindrop uses a folder structure, while Linkding uses a tag-based system (with a nifty feature called "Bundles" on top of that).

## Let's ask AI to automate this for us!

I spun up a local intance of Linkding (See [Linkding#Using Docker](https://linkding.link/installation/#using-docker)), mounted the data directory so I could have access to the SQLite database files, navigated into it and fired up Claude Code.

I had to give Claude Code all the necessary context so that it could successfully finish the task. I asked it to:

- Use a sqlite docker container to view the tables and structure of the Linkding database, in order to understand how the data and the relationship are stored.
- Read my Raindrop HTML export file containing all my bookmarks
- Look at how the bookmarks are organized in the Raindrop export file so it can understand the nesting of bookmarks inside collections (i.e. folders)
- Create a script to parse the Raindrop export file and generate a `.sql` file that will populate Linkding.

I also told it that all tags must be ASCII only, lowercase, without spaces (only dashes), and without special characters.

Claude generated the [following script](https://gist.github.com/orellazri/5e016500177e93d440071d5dffc418db).

This script assumes that it's running inside Linkding's data directory, located at `/data` (Important!), and there is a filed called "Raindrop Export.HTML" in that directory.

Running this script creates a new file called `organize_bookmarks.sql`

## ... Profit ?

Now we can run the script:

```bash
docker run --rm -v $(pwd):/data python:3.11-slim python /data/parse_raindrop.py
```

Example output:

```
Found 500 bookmarks

Bookmark distribution by main folder:
  Software -> software: 100 bookmarks
  Misc -> misc: 50 bookmarks
  ...
```

After running the script, we now have a `organize_bookmarks.sql` file that we can run on Linkding's SQLite database to import all our data.

At this point, after all my local testing, I deployed Linkding on my actual server, and then copied over the `.sql` file to it for the real import.

First, stop the Linkding container:

```bash
docker stop linkding
```

Now, import the `.sql` file into the database:

```bash
docker run --rm -v $(pwd):/data \
  --user $(id -u www-data):$(id -g www-data) \
  keinos/sqlite3 sqlite3 /data/db.sqlite3 \
  '.read /data/organize_bookmarks.sql'
```

*Note: *It's important to run as the `www-data` user (which is the user owning the database files), otherwise you'll get the following error: `Runtime error near line 2135: attempt to write a readonly database (8)`.

And that's it!

## How Tags Are Organized

The script follows these rules:

1. **Tag normalization**: All tags are lowercase with dashes
   - "3D Printing" → `3d-printing`
   - "Mechanical Keyboards" → `mechanical-keyboards`

2. **Nested folders**: Only the deepest folder becomes a tag
   - Bookmark in "Software > Rust" → gets tag `rust`

3. **All tags are created**: Even parent folders become separate tags for organization
