---
title: {{ replace (.Name | replaceRE "^[0-9]{4}-[0-9]{2}-[0-9]{2}-" "") "-" " " | title }}
tags: []
date: {{ .Name | replaceRE "^([0-9]{4}-[0-9]{2}-[0-9]{2}).*" "$1" }}
slug: {{ .Name | replaceRE "^[0-9]{4}-[0-9]{2}-[0-9]{2}-" "" }}
toc: true
---
