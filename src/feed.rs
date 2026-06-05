use std::{fs, path::Path};

use anyhow::Result;
use atom_syndication::{ContentBuilder, EntryBuilder, FeedBuilder, LinkBuilder, PersonBuilder};
use chrono::{DateTime, NaiveTime, TimeZone, Utc};

use crate::models::Post;

pub fn write_atom(
    posts: &[Post],
    dist: &Path,
    base_url: &str,
    title: &str,
    author: &str,
) -> Result<()> {
    let author = PersonBuilder::default().name(author).build();

    let entries: Vec<_> = posts
        .iter()
        .take(20)
        .map(|p| {
            let url = format!("{}/posts/{}/", base_url, p.slug);
            let dt: DateTime<Utc> = Utc.from_utc_datetime(
                &p.front
                    .date
                    .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
            );

            EntryBuilder::default()
                .title(p.front.title.clone())
                .id(url.clone())
                .link(
                    LinkBuilder::default()
                        .href(url.clone())
                        .mime_type(Some("text/html".to_string()))
                        .build(),
                )
                .author(author.clone())
                .updated(dt)
                .content(
                    ContentBuilder::default()
                        .content_type(Some("html".to_string()))
                        .base(Some(url.to_string()))
                        .value(Some(p.html.clone()))
                        .build(),
                )
                .build()
        })
        .collect();

    let last_updated = match posts.first() {
        Some(p) => Utc.from_utc_datetime(
            &p.front
                .date
                .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
        ),
        None => Utc::now(),
    };

    let feed = FeedBuilder::default()
        .title(title)
        .lang(Some("en".to_string()))
        .id(format!("{}/atom.xml", base_url))
        .link(
            LinkBuilder::default()
                .href(format!("{}/", base_url))
                .mime_type(Some("text/html".to_string()))
                .build(),
        )
        .link(
            LinkBuilder::default()
                .href(format!("{}/atom.xml", base_url))
                .rel("self")
                .mime_type(Some("application/atom+xml".to_string()))
                .build(),
        )
        .author(author)
        .updated(last_updated)
        .entries(entries)
        .build();

    let file = fs::File::create(dist.join("atom.xml"))?;
    feed.write_to(file)?;

    Ok(())
}
