Rojo underwent a large refactor during most of 2018 to enable a bunch of new features and lay groundwork for lots more in 2019. As such, Rojo **0.5.x** projects are not compatible with Rojo **0.4.x** projects.

[TOC]

## Supporting Both 0.4.x and 0.5.x
Rojo 0.5.x uses a different name for its project format. While 0.4.x used `rojo.json`, 0.5.x uses `default.project.json`, which allows them to coexist.

If you aren't sure about upgrading or want to upgrade gradually, it's possible to keep both files in the same project without causing problems.

## Upgrading Your Project File
Project files in 0.5.x are more explicit and flexible than they were in 0.4.x. Project files can now describe models and plugins in addition to places.

This new project file format also guards against two of the biggest pitfalls when writing a config file:

* Using a service as a partition target directly, which often wiped away extra instances
* Defining two partitions that overlapped, which made Rojo act unpredictably

The biggest change is that the `partitions` field has been replaced with a new field, `tree`, that describes the entire hierarchy of your project from the top-down.

A project for 0.4.x that syncs from the `src` directory into `ReplicatedStorage.Source` would look like this:

```json
{
    "name": "Rojo 0.4.x Example",
    "partitions": {
        "path": "src",
        "target": "ReplicatedStorage.Source"
    }
}
```

In 0.5.x, the project format is more explicit:

```json
{
    "name": "Rojo 0.5.x Example",
    "tree": {
        "$className": "DataModel",
        "ReplicatedStorage": {
            "$className": "ReplicatedStorage",
            "Source": {
                "$path": "src"
            }
        }
    }
}
```

For each object in the tree, we define *metadata* and *children*.

Metadata begins with a dollar sign (`$`), like `$className`. This is so that children and metadata can coexist without creating too many nested layers.

All other values are considered children, where the key is the instance's name, and the value is an object, repeating the process.

## Migrating Unknown Files
If you used Rojo to sync in files as `StringValue` objects, you'll need to make sure those files end with the `txt` extension to preserve this in Rojo 0.5.x.

Unknown files are now ignored in Rojo instead of being converted to `StringValue` objects.

## Migrating `init.model.json` files
In Rojo 0.4.x, it's possible to create a file named `init.model.json` that lets you describe a model that becomes the container for all of the other files in the folder, just like `init.lua`.

In Rojo 0.5.x, this feature has been replaced with `init.meta.json` files. See [Sync Details](../../reference/sync-details) for more information about these new files.
