<!-- image -->

<!-- image -->

## B8 Settings

General

Annotations

Variables

Links

Versions

Permissions

JSON Model

## YAPS Pushes

Name

YAPS Pushes

## Data source

misa grafana-mysal

## Enabled

When enabled the annotation query is issued every dashboard refresh

<!-- image -->

## Hidden

Annotation queries can be toggled on or off at the top of the dashboard. With this option checked this toggle will be hidden.

<!-- image -->

## Color

Color to use for the annotation event markers

<!-- image -->

Show in

All panels

## Query

Format: Table v

```
10 11 12 13 SELECT a. id, a.epoch as time, a.epoch_end as time_end, a. text, SUBSTRING (a. tags, 2, CHAR_LENGTH(a. tags) - 2) as tags FROM annotation a WHERE a.org_id AND a. epoch <= AND a.epoch_end >= '$__from' AND SELECT SUM(1) FROM annotation_tag at INNER JOIN tag on tag. id = at.tag_id WHERE at. annotation_id = a.id
```

Close

Save as

<!-- image -->

Save dashboard

<!-- image -->

<!-- image -->