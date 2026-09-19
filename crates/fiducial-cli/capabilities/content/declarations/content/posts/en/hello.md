---
title: Hello
date: 2026-01-01
summary: Replace this entry. It exists so the pipeline has something to derive.
tags: [example]
---

The frontmatter above is checked against `[collections.posts] schema` in
`content.toml`. A field that is not in the schema fails the build, and so does
a locale that is missing this file.
