# Rojo Changelog

## Unreleased

* Fixed a bug where the last sync timestamp was not updating correctly in the plugin ([#1132])
* Improved the reliability of sync replacements by adding better error handling and recovery ([#1135])

[#1132]: https://github.com/rojo-rbx/rojo/pull/1132
[#1135]: https://github.com/rojo-rbx/rojo/pull/1135

## 7.6.0 - October 10th, 2025
* Added flag to `rojo init` to skip initializing a git repository ([#1122])
* Added fallback method for when an Instance can't be synced through normal means ([#1030])
 	This should make it possible to sync `MeshParts` and `Unions`!

 	The fallback involves deleting and recreating Instances. This will break
 	properties that reference them that Rojo does not know about, so be weary.

* Add auto-reconnect and improve UX for sync reminders ([#1096])
* Add support for syncing `yml` and `yaml` files (behaves similar to JSON and TOML) ([#1093])
* Fixed colors of Table diff ([#1084])
* Fixed `sourcemap` command outputting paths with OS-specific path separators ([#1085])
* Fixed nil -> nil properties showing up as failing to sync in plugin's patch visualizer ([#1081])
* Changed the background of the server's in-browser UI to be gray instead of white ([#1080])
* Fixed `Auto Connect Playtest Server` no longer functioning due to Roblox change ([#1066])
* Added an update indicator to the version header when a new version of the plugin is available. ([#1069])
* Added `--absolute` flag to the sourcemap subcommand, which will emit absolute paths instead of relative paths. ([#1092])
* Fixed applying `gameId` and `placeId` before initial sync was accepted ([#1104])

[#1122]: https://github.com/rojo-rbx/rojo/pull/1122
[#1030]: https://github.com/rojo-rbx/rojo/pull/1030
[#1096]: https://github.com/rojo-rbx/rojo/pull/1096
[#1093]: https://github.com/rojo-rbx/rojo/pull/1093
[#1084]: https://github.com/rojo-rbx/rojo/pull/1084
[#1085]: https://github.com/rojo-rbx/rojo/pull/1085
[#1081]: https://github.com/rojo-rbx/rojo/pull/1081
[#1080]: https://github.com/rojo-rbx/rojo/pull/1080
[#1066]: https://github.com/rojo-rbx/rojo/pull/1066
[#1069]: https://github.com/rojo-rbx/rojo/pull/1069
[#1092]: https://github.com/rojo-rbx/rojo/pull/1092
[#1104]: https://github.com/rojo-rbx/rojo/pull/1104

## 7.5.1 - April 25th, 2025
* Fixed output spam related to `Instance.Capabilities` in the plugin

## 7.5.0 - April 25th, 2025
* Fixed an edge case that caused model pivots to not be built correctly in some cases ([#1027])
* Add `blockedPlaceIds` project config field to allow blocking place ids from being live synced ([#1021])
* Adds support for `.plugin.lua(u)` files - this applies the `Plugin` RunContext. ([#1008])
* Added support for Roblox's `Content` type. This replaces the old `Content` type with `ContentId` to reflect Roblox's change.
 	If you were previously using the fully-qualified syntax for `Content` you will need to switch it to `ContentId`.
* Added support for `Enum` attributes
* Significantly improved performance of `.rbxm` parsing
* Support for a `$schema` field in all special JSON files (`.project.json`, `.model.json`, and `.meta.json`) ([#974])
* Projects may now manually link `Ref` properties together using `Attributes`. ([#843])
 	This has two parts: using `id` or `$id` in JSON files or a `Rojo_Target` attribute, an Instance
    is given an ID. Then, that ID may be used elsewhere in the project to point to an Instance
    using an attribute named `Rojo_Target_PROP_NAME`, where `PROP_NAME` is the name of a property.

    As an example, here is a `model.json` for an ObjectValue that refers to itself:

    ```json
    {
        "id": "arbitrary string",
        "attributes": {
            "Rojo_Target_Value": "arbitrary string"
        }
    }
    ```

    This is a very rough implementation and the usage will become more ergonomic
    over time.

* Updated Undo/Redo history to be more robust ([#915])
* Added popout diff visualizer for table properties like Attributes and Tags ([#834])
* Updated Theme to use Studio colors ([#838])
* Improved patch visualizer UX ([#883])
* Added update notifications for newer compatible versions in the Studio plugin. ([#832])
* Added experimental setting for Auto Connect in playtests ([#840])
* Improved settings UI ([#886])
* `Open Scripts Externally` option can now be changed while syncing ([#911])
* The sync reminder notification will now tell you what was last synced and when ([#987])
* Fixed notification and tooltip text sometimes getting cut off ([#988])
* Projects may now specify rules for syncing files as if they had a different file extension. ([#813])
 	This is specified via a new field on project files, `syncRules`:

 	```json
 	{
 	 	"syncRules": [
 	 	 	{
 	 	 	 	"pattern": "*.foo",
 	 	 	 	"use": "text",
                "exclude": "*.exclude.foo",
 	 	 	},
 	 	 	{
 	 	 	 	"pattern": "*.bar.baz",
 	 	 	 	"use": "json",
 	 	 	 	"suffix": ".bar.baz",
 	 	 	},
 	 	],
 	 	"name": "SyncRulesAreCool",
 	 	"tree": {
 	 	 	"$path": "src"
 	 	}
 	}
 	```

 	The `pattern` field is a glob used to match the sync rule to files. If present, the `suffix` field allows you to specify parts of a file's name get cut off by Rojo to name the Instance, including the file extension. If it isn't specified, Rojo will only cut off the first part of the file extension, up to the first dot.

    Additionally, the `exclude` field allows files to be excluded from the sync rule if they match a pattern specified by it. If it's not present, all files that match `pattern` will be modified using the sync rule.

 	The `use` field corresponds to one of the potential file type that Rojo will currently include in a project. Files that match the provided pattern will be treated as if they had the file extension for that file type.

 	| `use` value    | file extension  |
 	|:---------------|:----------------|
 	| `serverScript` | `.server.lua`   |
 	| `clientScript` | `.client.lua`   |
 	| `moduleScript` | `.lua`          |
 	| `json`         | `.json`         |
 	| `toml`         | `.toml`         |
 	| `csv`          | `.csv`          |
 	| `text`         | `.txt`          |
 	| `jsonModel`    | `.model.json`   |
 	| `rbxm`         | `.rbxm`         |
 	| `rbxmx`        | `.rbxmx`        |
 	| `project`      | `.project.json` |
 	| `ignore`       | None!           |

	Additionally, there are `use` values for specific script types ([#909]):

	| `use` value              | script type                            |
	|:-------------------------|:---------------------------------------|
	| `legacyServerScript`     | `Script` with `Enum.RunContext.Legacy` |
	| `legacyClientScript`     | `LocalScript`                          |
	| `runContextServerScript` | `Script` with `Enum.RunContext.Server` |
	| `runContextClientScript` | `Script` with `Enum.RunContext.Client` |
    | `pluginScript`           | `Script` with `Enum.RunContext.Plugin` |

    **All** sync rules are reset between project files, so they must be specified in each one when nesting them. This is to ensure that nothing can break other projects by changing how files are synced!

[#813]: https://github.com/rojo-rbx/rojo/pull/813
[#832]: https://github.com/rojo-rbx/rojo/pull/832
[#834]: https://github.com/rojo-rbx/rojo/pull/834
[#838]: https://github.com/rojo-rbx/rojo/pull/838
[#840]: https://github.com/rojo-rbx/rojo/pull/840
[#843]: https://github.com/rojo-rbx/rojo/pull/843
[#883]: https://github.com/rojo-rbx/rojo/pull/883
[#886]: https://github.com/rojo-rbx/rojo/pull/886
[#909]: https://github.com/rojo-rbx/rojo/pull/909
[#911]: https://github.com/rojo-rbx/rojo/pull/911
[#915]: https://github.com/rojo-rbx/rojo/pull/915
[#974]: https://github.com/rojo-rbx/rojo/pull/974
[#987]: https://github.com/rojo-rbx/rojo/pull/987
[#988]: https://github.com/rojo-rbx/rojo/pull/988
[#1008]: https://github.com/rojo-rbx/rojo/pull/1008
[#1021]: https://github.com/rojo-rbx/rojo/pull/1021
[#1027]: https://github.com/rojo-rbx/rojo/pull/1027

## [7.4.4] - August 22nd, 2024
* Fixed issue with reading attributes from `Lighting` in new place files
* `Instance.Archivable` will now default to `true` when building a project into a binary (`rbxm`/`rbxl`) file rather than `false`.

## [7.4.3] - August 6th, 2024
* Fixed issue with building binary files introduced in 7.4.2
* Fixed `value of type nil cannot be converted to number` warning spam in output. [#955]

[#955]: https://github.com/rojo-rbx/rojo/pull/955

## [7.4.2] - July 23, 2024
* Added Never option to Confirmation ([#893])
* Fixed removing trailing newlines ([#903])
* Updated the internal property database, correcting an issue with `SurfaceAppearance.Color` that was reported [here][Surface_Appearance_Color_1] and [here][Surface_Appearance_Color_2] ([#948])

[#893]: https://github.com/rojo-rbx/rojo/pull/893
[#903]: https://github.com/rojo-rbx/rojo/pull/903
[#948]: https://github.com/rojo-rbx/rojo/pull/948
[Surface_Appearance_Color_1]: https://devforum.roblox.com/t/jailbreak-custom-character-turned-shiny-black-no-texture/3075563
[Surface_Appearance_Color_2]: https://devforum.roblox.com/t/surfaceappearance-not-displaying-correctly/3075588

## [7.4.1] - February 20, 2024
* Made the `name` field optional on project files ([#870])

	Files named `default.project.json` inherit the name of the folder they're in and all other projects
    are named as expect (e.g. `foo.project.json` becomes an Instance named `foo`)

    There is no change in behavior if `name` is set.
* Fixed incorrect results when building model pivots ([#865])
* Fixed incorrect results when serving model pivots ([#868])
* Rojo now converts any line endings to LF, preventing spurious diffs when syncing Lua files on Windows ([#854])
* Fixed Rojo plugin failing to connect when project contains certain unreadable properties ([#848])
* Fixed various cases where patch visualizer would not display sync failures ([#845], [#844])
* Fixed http error handling so Rojo can be used in Github Codespaces ([#847])

[#848]: https://github.com/rojo-rbx/rojo/pull/848
[#845]: https://github.com/rojo-rbx/rojo/pull/845
[#844]: https://github.com/rojo-rbx/rojo/pull/844
[#847]: https://github.com/rojo-rbx/rojo/pull/847
[#854]: https://github.com/rojo-rbx/rojo/pull/854
[#865]: https://github.com/rojo-rbx/rojo/pull/865
[#868]: https://github.com/rojo-rbx/rojo/pull/868
[#870]: https://github.com/rojo-rbx/rojo/pull/870

## [7.4.0] - January 16, 2024
* Improved the visualization for array properties like Tags ([#829])
* Significantly improved performance of `rojo serve`, `rojo build --watch`, and `rojo sourcemap --watch` on macOS. ([#830])
* Changed *.lua files that init command generates to *.luau ([#831])
* Does not remind users to sync if the sync lock is claimed already ([#833])

[#829]: https://github.com/rojo-rbx/rojo/pull/829
[#830]: https://github.com/rojo-rbx/rojo/pull/830
[#831]: https://github.com/rojo-rbx/rojo/pull/831
[#833]: https://github.com/rojo-rbx/rojo/pull/833

## [7.4.0-rc3] - October 25, 2023
* Changed `sourcemap --watch` to only generate the sourcemap when it's necessary ([#800])
* Switched script source property getter and setter to `ScriptEditorService` methods ([#801])

 	This ensures that the script editor reflects any changes Rojo makes to a script while it is open in the script editor.

* Fixed issues when handling `SecurityCapabilities` values ([#803], [#807])
* Fixed Rojo plugin erroring out when attempting to sync attributes with invalid names ([#809])

[#800]: https://github.com/rojo-rbx/rojo/pull/800
[#801]: https://github.com/rojo-rbx/rojo/pull/801
[#803]: https://github.com/rojo-rbx/rojo/pull/803
[#807]: https://github.com/rojo-rbx/rojo/pull/807
[#809]: https://github.com/rojo-rbx/rojo/pull/809

## [7.4.0-rc2] - October 3, 2023
* Fixed bug with parsing version for plugin validation ([#797])

[#797]: https://github.com/rojo-rbx/rojo/pull/797

## [7.4.0-rc1] - October 3, 2023
### Additions
#### Project format
* Added support for `.toml` files to `$path` ([#633])
* Added support for `Font` and `CFrame` attributes ([rbx-dom#299], [rbx-dom#296])
* Added the `emitLegacyScripts` field to the project format ([#765]). The behavior is outlined below:

	| `emitLegacyScripts` Value | Action Taken by Rojo                                                                                             |
	|---------------------------|------------------------------------------------------------------------------------------------------------------|
	| false                     | Rojo emits Scripts with the appropriate `RunContext` for `*.client.lua` and `*.server.lua` files in the project. |
	| true   (default)          | Rojo emits LocalScripts and Scripts with legacy `RunContext` (same behavior as previously).                      |


	It can be used like this:
	```json
	{
		"emitLegacyScripts": false,
		"name": "MyCoolRunContextProject",
		"tree": {
			"$path": "src"
		}
	}
	```

* Added `Terrain` classname inference, similar to services ([#771])

	`Terrain` may now be defined in projects without using `$className`:
	```json
	"Workspace": {
		"Terrain": {
			"$path": "path/to/terrain.rbxm"
		}
	}
	```

* Added support for `Terrain.MaterialColors` ([#770])

	`Terrain.MaterialColors` is now represented in projects in a human readable format:
	```json
	"Workspace": {
		"Terrain": {
			"$path": "path/to/terrain.rbxm"
			"$properties": {
				"MaterialColors": {
					"Grass": [10, 20, 30],
					"Asphalt": [40, 50, 60],
					"LeafyGrass": [255, 155, 55]
				}
			}
		}
	}
	```

* Added better support for `Font` properties ([#731])

	`FontFace` properties may now be defined using implicit property syntax:
	```json
	"TextBox": {
		"$className": "TextBox",
		"$properties": {
			"FontFace": {
				"family": "rbxasset://fonts/families/RobotoMono.json",
				"weight": "Thin",
				"style": "Normal"
			}
		}
	}
	```

#### Patch visualizer and notifications
* Added a setting to control patch confirmation behavior ([#774])

	This is a new setting for controlling when the Rojo plugin prompts for confirmation before syncing. It has four options:
    * Initial (default): prompts only once for a project in a given Studio session
    * Always: always prompts for confirmation
    * Large Changes: only prompts when there are more than X changed instances. The number of instances is configurable - an additional setting for the number of instances becomes available when this option is chosen
    * Unlisted PlaceId: only prompts if the place ID is not present in servePlaceIds

* Added the ability to select Instances in patch visualizer ([#709])

	Double-clicking an instance in the patch visualizer sets Roblox Studio's selection to the instance.

* Added a sync reminder notification. ([#689])

	Rojo detects if you have previously synced to a place, and displays a notification reminding you to sync again:

	![Rojo reminds you to sync a place that you've synced previously](https://user-images.githubusercontent.com/40185666/242397435-ccdfddf2-a63f-420c-bc18-a6e3d6455bba.png)

* Added rich Source diffs in patch visualizer ([#748])

	A "View Diff" button for script sources is now present in the patch visualizer. Clicking it displays a side-by-side diff of the script changes:

	![The patch visualizer contains a "view diff" button](https://user-images.githubusercontent.com/40185666/256065992-3f03558f-84b0-45a1-80eb-901f348cf067.png)

	![The "View Diff" button opens a widget that displays a diff](https://user-images.githubusercontent.com/40185666/256066084-1d9d8fe8-7dad-4ee7-a542-b4aee35a5644.png)

* Patch visualizer now indicates what changes failed to apply. ([#717])

	A clickable warning label is displayed when the Rojo plugin is unable to apply changes. Clicking the label displays precise information about which changes failed:

	![Patch visualizer displays a clickable warning label when changes fail to apply](https://user-images.githubusercontent.com/40185666/252063660-f08399ef-1e16-4f1c-bed8-552821f98cef.png)


#### Miscellaneous
* Added `plugin` flag to the `build` command that outputs to the local plugins folder ([#735])

	This is a flag that builds a Rojo project into Roblox Studio's plugins directory. This allows you to build a Rojo project and load it into Studio as a plugin without having to type the full path to the plugins directory. It can be used like this: `rojo build <PATH-TO-PROJECT> --plugin <FILE-NAME>`

* Added new plugin template to the `init` command ([#738])

	This is a new template geared towards plugins. It is similar to the model template, but creates a `Script` instead of a `ModuleScript` in the `src` directory. It can be used like this: `rojo init --kind plugin`

* Added protection against syncing non-place projects as a place. ([#691])
* Add buttons for navigation on the Connected page ([#722])

### Fixes
* Significantly improved performance of `rojo sourcemap` ([#668])
* Fixed the diff visualizer of connected sessions. ([#674])
* Fixed disconnected session activity. ([#675])
* Skip confirming patches that contain only a datamodel name change. ([#688])
* Fix Rojo breaking when users undo/redo in Studio ([#708])
* Improve tooltip behavior ([#723])
* Better settings controls ([#725])
* Rework patch visualizer with many fixes and improvements ([#713], [#726], [#755])

[#668]: https://github.com/rojo-rbx/rojo/pull/668
[#674]: https://github.com/rojo-rbx/rojo/pull/674
[#675]: https://github.com/rojo-rbx/rojo/pull/675
[#688]: https://github.com/rojo-rbx/rojo/pull/688
[#689]: https://github.com/rojo-rbx/rojo/pull/689
[#691]: https://github.com/rojo-rbx/rojo/pull/691
[#709]: https://github.com/rojo-rbx/rojo/pull/709
[#708]: https://github.com/rojo-rbx/rojo/pull/708
[#713]: https://github.com/rojo-rbx/rojo/pull/713
[#717]: https://github.com/rojo-rbx/rojo/pull/717
[#722]: https://github.com/rojo-rbx/rojo/pull/722
[#723]: https://github.com/rojo-rbx/rojo/pull/723
[#725]: https://github.com/rojo-rbx/rojo/pull/725
[#726]: https://github.com/rojo-rbx/rojo/pull/726
[#633]: https://github.com/rojo-rbx/rojo/pull/633
[#735]: https://github.com/rojo-rbx/rojo/pull/735
[#731]: https://github.com/rojo-rbx/rojo/pull/731
[#738]: https://github.com/rojo-rbx/rojo/pull/738
[#748]: https://github.com/rojo-rbx/rojo/pull/748
[#755]: https://github.com/rojo-rbx/rojo/pull/755
[#765]: https://github.com/rojo-rbx/rojo/pull/765
[#770]: https://github.com/rojo-rbx/rojo/pull/770
[#771]: https://github.com/rojo-rbx/rojo/pull/771
[#774]: https://github.com/rojo-rbx/rojo/pull/774
[rbx-dom#299]: https://github.com/rojo-rbx/rbx-dom/pull/299
[rbx-dom#296]: https://github.com/rojo-rbx/rbx-dom/pull/296

## [7.3.0] - April 22, 2023
* Added `$attributes` to project format. ([#574])
* Added `--watch` flag to `rojo sourcemap`. ([#602])
* Added support for `init.csv` files. ([#594])
* Added real-time sync status to the Studio plugin. ([#569])
* Added support for copying error messages to the clipboard. ([#614])
* Added sync locking for Team Create. ([#590])
* Added support for specifying HTTP or HTTPS protocol in plugin. ([#642])
* Added tooltips to buttons in the Studio plugin. ([#637])
* Added visual diffs when connecting from the Studio plugin. ([#603])
* Host and port are now saved in the Studio plugin. ([#613])
* Improved padding on notifications in Studio plugin. ([#589])
* Renamed `Common` to `Shared` in the default Rojo project. ([#611])
* Reduced the minimum size of the Studio plugin widget. ([#606])
* Fixed current directory in `rojo fmt-project`. ([#581])
* Fixed errors after a session has already ended. ([#587])
* Fixed an uncommon security permission error ([#619])

[#569]: https://github.com/rojo-rbx/rojo/pull/569
[#574]: https://github.com/rojo-rbx/rojo/pull/574
[#581]: https://github.com/rojo-rbx/rojo/pull/581
[#587]: https://github.com/rojo-rbx/rojo/pull/587
[#589]: https://github.com/rojo-rbx/rojo/pull/589
[#590]: https://github.com/rojo-rbx/rojo/pull/590
[#594]: https://github.com/rojo-rbx/rojo/pull/594
[#602]: https://github.com/rojo-rbx/rojo/pull/602
[#603]: https://github.com/rojo-rbx/rojo/pull/603
[#606]: https://github.com/rojo-rbx/rojo/pull/606
[#611]: https://github.com/rojo-rbx/rojo/pull/611
[#613]: https://github.com/rojo-rbx/rojo/pull/613
[#614]: https://github.com/rojo-rbx/rojo/pull/614
[#619]: https://github.com/rojo-rbx/rojo/pull/619
[#637]: https://github.com/rojo-rbx/rojo/pull/637
[#642]: https://github.com/rojo-rbx/rojo/pull/642
[7.3.0]: https://github.com/rojo-rbx/rojo/releases/tag/v7.3.0

## [7.2.1] - July 8, 2022
* Fixed notification sound by changing it to a generic sound. ([#566])
* Added setting to turn off sound effects. ([#568])

[#566]: https://github.com/rojo-rbx/rojo/pull/566
[#568]: https://github.com/rojo-rbx/rojo/pull/568
[7.2.1]: https://github.com/rojo-rbx/rojo/releases/tag/v7.2.1

## [7.2.0] - June 29, 2022
* Added support for `.luau` files. ([#552])
* Added support for live syncing Attributes and Tags. ([#553])
* Added notification popups in the Roblox Studio plugin. ([#540])
* Fixed `init.meta.json` when used with `init.lua` and related files. ([#549])
* Fixed incorrect output when serving from a non-default address or port ([#556])
* Fixed Linux binaries not running on systems with older glibc. ([#561])
* Added `camelCase` casing for JSON models, deprecating `PascalCase` names. ([#563])
* Switched from structopt to clap for command line argument parsing.
* Significantly improved performance of building and serving. ([#548])
* Increased minimum supported Rust version to 1.57.0. ([#564])

[#540]: https://github.com/rojo-rbx/rojo/pull/540
[#548]: https://github.com/rojo-rbx/rojo/pull/548
[#549]: https://github.com/rojo-rbx/rojo/pull/549
[#552]: https://github.com/rojo-rbx/rojo/pull/552
[#553]: https://github.com/rojo-rbx/rojo/pull/553
[#556]: https://github.com/rojo-rbx/rojo/pull/556
[#561]: https://github.com/rojo-rbx/rojo/pull/561
[#563]: https://github.com/rojo-rbx/rojo/pull/563
[#564]: https://github.com/rojo-rbx/rojo/pull/564
[7.2.0]: https://github.com/rojo-rbx/rojo/releases/tag/v7.2.0

## [7.1.1] - May 26, 2022
* Fixed sourcemap command not stripping paths correctly ([#544])
* Fixed Studio plugin settings not saving correctly.

[#544]: https://github.com/rojo-rbx/rojo/pull/544
[#545]: https://github.com/rojo-rbx/rojo/pull/545
[7.1.1]: https://github.com/rojo-rbx/rojo/releases/tag/v7.1.1

## [7.1.0] - May 22, 2022
* Added support for specifying an address to be used by default in project files. ([#507])
* Added support for optional paths in project files. ([#472])
* Added support for the new Open Cloud API when uploading. ([#504])
* Added `sourcemap` command for generating sourcemaps to feed into other tools. ([#530])
* Added PluginActions for connecting/disconnecting a session ([#537])
* Added changing toolbar icon to indicate state ([#538])

[#472]: https://github.com/rojo-rbx/rojo/pull/472
[#504]: https://github.com/rojo-rbx/rojo/pull/504
[#507]: https://github.com/rojo-rbx/rojo/pull/507
[#530]: https://github.com/rojo-rbx/rojo/pull/530
[#537]: https://github.com/rojo-rbx/rojo/pull/537
[#538]: https://github.com/rojo-rbx/rojo/pull/538
[7.1.0]: https://github.com/rojo-rbx/rojo/releases/tag/v7.1.0

## [7.0.0] - December 10, 2021
* Fixed Rojo's interactions with properties enabled by FFlags that are not yet enabled. ([#493])
* Improved output in Roblox Studio plugin when bad property data is encountered.
* Reintroduced support for CFrame shorthand syntax in Rojo project and `.meta.json` files, matching Rojo 6. ([#430])
* Connection settings are now remembered when reconnecting in Roblox Studio. ([#500])
* Updated reflection database to Roblox v503.

[#430]: https://github.com/rojo-rbx/rojo/issues/430
[#493]: https://github.com/rojo-rbx/rojo/pull/493
[#500]: https://github.com/rojo-rbx/rojo/pull/500
[7.0.0]: https://github.com/rojo-rbx/rojo/releases/tag/v7.0.0

## [7.0.0-rc.3] - October 19, 2021
This is the last release candidate for Rojo 7. In an effort to get Rojo 7 out the door, we'll be freezing features from here on out, something we should've done a couple months ago.

Expect to see Rojo 7 stable soon!

* Added support for writing `Tags` in project files, model files, and meta files. ([#484])
* Adjusted Studio plugin colors to match Roblox Studio palette. ([#482])
* Improved experimental two-way sync feature by batching changes. ([#478])

[#482]: https://github.com/rojo-rbx/rojo/pull/482
[#484]: https://github.com/rojo-rbx/rojo/pull/484
[#478]: https://github.com/rojo-rbx/rojo/pull/478
[7.0.0-rc.3]: https://github.com/rojo-rbx/rojo/releases/tag/v7.0.0-rc.3

## 7.0.0-rc.2 - October 19, 2021
(Botched release due to Git mishap, oops!)

## [7.0.0-rc.1] - August 23, 2021
In Rojo 6 and previous Rojo 7 alphas, an explicit Vector3 property would be written like this:

```json
{
    "className": "Part",
    "properties": {
        "Position": {
            "Type": "Vector3",
            "Value": [1, 2, 3]
        }
    }
}
```

For Rojo 7, this will need to be changed to:

```json
{
    "className": "Part",
    "properties": {
        "Position": {
            "Vector3": [1, 2, 3]
        }
    }
}
```

The shorthand property format that most users use is not impacted. For reference, it looks like this:

```json
{
    "className": "Part",
    "properties": {
        "Position": [1, 2, 3]
    }
}
```

* Major breaking change: changed property syntax for project files; shorthand syntax is unchanged.
* Added the `fmt-project` subcommand for formatting Rojo project files.
* Improved error output for many subcommands.
* Updated to stable versions of rbx-dom libraries.
* Updated async infrastructure, which should fix a handful of bugs. ([#459])
* Fixed syncing refs in the Roblox Studio plugin ([#462], [#466])
* Added support for long paths on Windows. ([#464])

[#459]: https://github.com/rojo-rbx/rojo/pull/459
[#462]: https://github.com/rojo-rbx/rojo/pull/462
[#464]: https://github.com/rojo-rbx/rojo/pull/464
[#466]: https://github.com/rojo-rbx/rojo/pull/466
[7.0.0-rc.1]: https://github.com/rojo-rbx/rojo/releases/tag/v7.0.0-rc.1

## [7.0.0-alpha.4][7.0.0-alpha.4] (May 5, 2021)
* Added the `gameId` and `placeId` optional properties to project files.
    * When connecting from the Rojo Roblox Studio plugin, Rojo will set the game and place ID of the current place to these values, if set.
    * This is equivalent to running `game:SetUniverseId(...)` and `game:SetPlaceId(...)` from the command bar in Studio.
* Added "EXPERIMENTAL!" label to two-way sync toggle in Rojo's Roblox Studio plugin.
* Fixed `Name` and `Parent` properties being allowed in Rojo projects. ([#413][pr-413])
* Fixed "Open Scripts Externally" feature crashing Studio. ([#369][issue-369])
* Empty `.model.json` files will no longer cause errors. ([#420][pr-420])
* When specifying `$path` on a service, Rojo now keeps the correct class name. ([#331][issue-331])
* Improved error messages for misconfigured projects.

[issue-331]: https://github.com/rojo-rbx/rojo/issues/331
[issue-369]: https://github.com/rojo-rbx/rojo/issues/369
[pr-420]: https://github.com/rojo-rbx/rojo/pull/420
[pr-413]: https://github.com/rojo-rbx/rojo/pull/413
[7.0.0-alpha.4]: https://github.com/rojo-rbx/rojo/releases/tag/v7.0.0-alpha.4

## [7.0.0-alpha.3][7.0.0-alpha.3] (February 19, 2021)
* Updated dependencies, fixing `OptionalCoordinateFrame`-related issues.
* Added `--address` flag to `rojo serve` to allow for external connections. ([#403][pr-403])

[pr-403]: https://github.com/rojo-rbx/rojo/pull/403
[7.0.0-alpha.3]: https://github.com/rojo-rbx/rojo/releases/tag/v7.0.0-alpha.3

## [7.0.0-alpha.2][7.0.0-alpha.2] (February 19, 2021)
* Fixed incorrect protocol version between the client and server.

[7.0.0-alpha.2]: https://github.com/rojo-rbx/rojo/releases/tag/v7.0.0-alpha.2

## [7.0.0-alpha.1][7.0.0-alpha.1] (February 18, 2021)
This release includes a brand new implementation of the Roblox DOM. It brings performance improvements, much better support for `rbxl` and `rbxm` files, and a better internal API.

* Added support for all remaining property types.
* Added support for the entire Roblox binary model format.
* Changed `rojo upload` to upload binary places and models instead of XML.
    * This should make using `rojo upload` much more feasible for large places.
* **Breaking**: Changed format of some types of values in `project.json`, `model.json`, and `meta.json` files.
    * This should impact few projects. See [this file][allValues.json] for new examples of each property type.

Formatting of types will change more before the stable release of Rojo 7. We're hoping to use this opportunity to normalize some of the case inconsistency introduced in Rojo 0.5.

[7.0.0-alpha.1]: https://github.com/rojo-rbx/rojo/releases/tag/v7.0.0-alpha.1
[allValues.json]: https://github.com/rojo-rbx/rojo/blob/f4a790eb50b74e482000bad1dcfe22533992fb20/plugin/rbx_dom_lua/src/allValues.json

## [6.0.2](https://github.com/rojo-rbx/rojo/releases/tag/v6.0.2) (February 9, 2021)
* Fixed `rojo upload` to handle CSRF challenges.

## [6.0.1](https://github.com/rojo-rbx/rojo/releases/tag/v6.0.1) (January 22, 2021)
* Fixed `rojo upload` requests being rejected by Roblox

## [6.0.0](https://github.com/rojo-rbx/rojo/releases/tag/v6.0.0) (January 16, 2021)
* Improved server error messages
    * The server will now keep running in more error cases
* Fixed Rojo being unable to diff ClassName changes

## [6.0.0 Release Candidate 4](https://github.com/rojo-rbx/rojo/releases/tag/v6.0.0-rc.4) (December 14, 2020)
* Added brand new Rojo UI ([#367](https://github.com/rojo-rbx/rojo/pull/367))
* Added `projectName` to `/api/rojo` output.

## [6.0.0 Release Candidate 3](https://github.com/rojo-rbx/rojo/releases/tag/v6.0.0-rc.3) (November 19, 2020)
* Fixed the Rojo plugin attempting to write the non-scriptable properties `Instance.SourceAssetId` and `HttpServer.HttpEnabled`.
* Fixed the Rojo plugin's handling of null referents.

## [6.0.0 Release Candidate 2](https://github.com/rojo-rbx/rojo/releases/tag/v6.0.0-rc.2) (November 19, 2020)
* Fixed crash when malformed CSV files are put into a project. ([#310](https://github.com/rojo-rbx/rojo/issues/310))
* Fixed incorrect string escaping when producing Lua code from JSON files. ([#314](https://github.com/rojo-rbx/rojo/issues/314))
* Fixed performance issues introduced in Rojo 6.0.0-rc.1. ([#317](https://github.com/rojo-rbx/rojo/issues/317))
* Fixed `rojo plugin install` subcommand failing for everyone except Rojo developers. ([#320](https://github.com/rojo-rbx/rojo/issues/320))
* Updated default place template to take advantage of [#210](https://github.com/rojo-rbx/rojo/pull/210).
* Enabled glob ignore patterns by default and removed the `unstable_glob_ignore` feature.
    * `globIgnorePaths` can be set on a project to a list of globs to ignore.
* The Rojo plugin now completes as much as it can from a patch without disconnecting. Warnings are shown in the console.
* Fixed 6.0.0-rc.1 regression causing instances that changed ClassName to instead... not change ClassName.

## [6.0.0 Release Candidate 1](https://github.com/rojo-rbx/rojo/releases/tag/v6.0.0-rc.1) (March 29, 2020)
This release jumped from 0.6.0 to 6.0.0. Rojo has been in use in production for many users for quite a long times, and so 6.0 is a more accurate reflection of Rojo's version than a pre-1.0 version.

* Added basic settings panel to plugin, with two settings:
    * "Open Scripts Externally": When enabled, opening a script in Studio will instead open it in your default text editor.
    * "Two-Way Sync": When enabled, Rojo will attempt to save changes to your place back to the filesystem. **Very early feature, very broken, beware!**
* Added `--color` option to force-enable or force-disable color in Rojo's output.
* Added support for turning `.json` files into `ModuleScript` instances ([#308](https://github.com/rojo-rbx/rojo/pull/308))
* Added `rojo plugin install` and `rojo plugin uninstall` to allow Rojo to manage its Roblox Studio plugin. ([#304](https://github.com/rojo-rbx/rojo/pull/304))
* Class names no longer need to be specified for Roblox services in Rojo projects. ([#210](https://github.com/rojo-rbx/rojo/pull/210))
* The server half of **experimental** two-way sync is now enabled by default.
* Increased default logging verbosity in commands like `rojo build`.
* Rojo now requires a project file again, just like 0.5.4.

## [0.6.0 Alpha 3](https://github.com/rojo-rbx/rojo/releases/tag/v0.6.0-alpha.3) (March 13, 2020)
* Added `--watch` argument to `rojo build`. ([#284](https://github.com/rojo-rbx/rojo/pull/284))
* Added dark theme support to plugin. ([#241](https://github.com/rojo-rbx/rojo/issues/241))
* Added a revamped `rojo init` command, which will now create more complete projects.
* Added the `rojo doc` command, which opens Rojo's documentation in your browser.
* Fixed many crashes from malformed projects and filesystem edge cases in `rojo serve`.
* Simplified filesystem access code dramatically.
* Improved error reporting and logging across the board.
    * Log messages have a less noisy prefix.
    * Any thread panicking now causes Rojo to abort instead of existing as a zombie.
    * Errors now have a list of causes, helping make many errors more clear.

## [0.6.0 Alpha 2](https://github.com/rojo-rbx/rojo/releases/tag/v0.6.0-alpha.2) (March 6, 2020)
* Fixed `rojo upload` command always uploading models.
* Removed `--kind` parameter to `rojo upload`; Rojo now automatically uploads the correct kind of asset based on your project file.

## [0.5.4](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.4) (February 26, 2020)
This is a general maintenance release for the Rojo 0.5.x release series.

* Updated reflection database and other dependencies.
* First stable release with binaries for macOS and Linux.

## [0.6.0 Alpha 1](https://github.com/rojo-rbx/rojo/releases/tag/v0.6.0-alpha.1) (January 22, 2020)

### General
* Added support for nested project files. ([#95](https://github.com/rojo-rbx/rojo/issues/95))
* Added project file hot-reloading. ([#10](https://github.com/rojo-rbx/rojo/issues/10)])
* Fixed Rojo dropping Ref properties ([#142](https://github.com/rojo-rbx/rojo/issues/142))
    * This means that properties like `PrimaryPart` now work!
* Improved live sync protocol to reduce round-trips and improve syncing consistency.
* Improved support for binary model files and places.

### Command Line
* Added `--verbose`/`-v` flag, which can be specified multiple times to increase verbosity.
* Added support for automatically finding Roblox Studio's auth cookie for `rojo upload` on Windows.
* Added support for building, serving and uploading sources that aren't Rojo projects.
* Improved feedback from `rojo serve`.
* Removed support for legacy `roblox-project.json` projects, deprecated in an early Rojo 0.5.0 alpha.
* Rojo no longer traverses directories upwards looking for project files.
    * Though undocumented, Rojo 0.5.x will search for a project file contained in any ancestor folders. This feature was removed to better support other 0.6.x features.

### Roblox Studio Plugin
* Added "connecting" state to improve experience when live syncing.
* Added "error" state to show errors in a place that isn't the output panel.
* Improved diagnostics for when the Rojo plugin cannot create an instance.

## [0.5.3](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.3) (October 15, 2019)
* Fixed an issue where Rojo would throw an error when encountering recently-added instance classes.

## [0.5.2](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.2) (October 14, 2019)
* Fixed an issue where `LocalizationTable` instances would have their column order randomized. ([#173](https://github.com/rojo-rbx/rojo/issues/173))

## [0.5.1](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.1) (October 4, 2019)
* Fixed an issue where Rojo would drop changes if they happened too quickly ([#252](https://github.com/rojo-rbx/rojo/issues/252))
* Improved diagnostics for when the Rojo plugin cannot create an instance.
* Updated dependencies
    * This brings Rojo's reflection database from client release 395 to client release 404.

## [0.5.0](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0) (August 27, 2019)
* Changed `.model.json` naming, which may require projects to migrate ambiguous cases:
    * The file name now takes precedence over the `Name` field in the model, like Rojo 0.4.x.
    * The `Name` field of the top-level instance is now optional. It's recommended that you remove it from your models.
    * Rojo will emit a warning when `Name` is specified and does not match the name from the file.
* Fixed `Rect` values being set to `0, 0, 0, 0` when synced with the Rojo plugin. ([#201](https://github.com/rojo-rbx/rojo/issues/201))
* Fixed live-syncing of `PhysicalProperties`, `NumberSequence`, and `ColorSequence` values

## [0.5.0 Alpha 13](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.13) (August 2, 2019)
* Bumped minimum Rust version to 1.34.0.
* Fixed default port documentation in `rojo serve --help` ([#219](https://github.com/rojo-rbx/rojo/issues/219))
* Fixed BrickColor support by upgrading Roblox-related dependencies

## [0.5.0 Alpha 12](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.12) (July 2, 2019)
* Added `.meta.json` files
    * `init.meta.json` files replace `init.model.json` files from Rojo 0.4.x ([#183](https://github.com/rojo-rbx/rojo/pull/183))
    * Other `.meta.json` files allow attaching extra data to other files ([#189](https://github.com/rojo-rbx/rojo/pull/189))
* Added support for infinite and NaN values in types like `Vector2` when building models and places.
    * These types aren't supported for live-syncing yet due to limitations around JSON encoding.
* Added support for using `SharedString` values when building XML models and places.
* Added support for live-syncing `CollectionService` tags.
* Added a warning when building binary place files, since they're still experimental and have bugs.
* Added a warning when trying to use Rojo 0.5.x with a Rojo 0.4.x-only project.
* Added a warning when a Rojo project contains keys that start with `$`, which are reserved names. ([#191](https://github.com/rojo-rbx/rojo/issues/191))
* Rojo now throws an error if unknown keys are found most files.
* Added an icon to the plugin's toolbar button
* Changed the plugin to use a docking widget for all UI.
* Changed the plugin to ignore unknown properties when live-syncing.
    * Rojo's approach to this problem might change later, like with a strict model mode ([#190](https://github.com/rojo-rbx/rojo/issues/190)) or another approach.
* Upgraded to reflection database from client release 388.
* Updated Rojo's branding to shift the color palette to make it work better on dark backgrounds

## [0.5.0 Alpha 11](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.11) (May 29, 2019)
* Added support for implicit property values in JSON model files ([#154](https://github.com/rojo-rbx/rojo/pull/154))
* `Content` properties can now be specified in projects and model files as regular string literals.
* Added support for `BrickColor` properties.
* Added support for properties added in client release 384, like `Lighting.Technology` being set to `"ShadowMap"`.
* Improved performance when working with XML models and places
* Fixed serializing empty `Content` properties as XML
* Fixed serializing infinite and NaN floating point properties in XML
* Improved compatibility with XML models
* Plugin should now be able to live-sync more properties, and ignore ones it can't, like `Lighting.Technology`.

## 0.5.0 Alpha 10
* This release was a dud due to [issue #176](https://github.com/rojo-rbx/rojo/issues/176) and was rolled back.

## [0.5.0 Alpha 9](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.9) (April 4, 2019)
* Changed `rojo build` to use buffered I/O, which can make it up to 2x faster in some cases.
    * Building [*Road Not Taken*](https://github.com/LPGhatguy/roads) to an `rbxlx` file dropped from 150ms to 70ms on my machine
* Fixed `LocalizationTable` instances being made from `csv` files incorrectly interpreting empty rows and columns. ([#149](https://github.com/rojo-rbx/rojo/pull/149))
* Fixed CSV files with entries that parse as numbers causing Rojo to panic. ([#152](https://github.com/rojo-rbx/rojo/pull/152))
* Improved error messages when malformed CSV files are found in a Rojo project.

## [0.5.0 Alpha 8](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.8) (March 29, 2019)
* Added support for a bunch of new types when dealing with XML model/place files:
    * `ColorSequence`
    * `Float64`
    * `Int64`
    * `NumberRange`
    * `NumberSequence`
    * `PhysicalProperties`
    * `Ray`
    * `Rect`
    * `Ref`
* Improved server instance ordering behavior when files are added during a live session ([#135](https://github.com/rojo-rbx/rojo/pull/135))
* Fixed error being thrown when trying to unload the Rojo plugin.
* Added partial fix for [issue #141](https://github.com/rojo-rbx/rojo/issues/141) for `Lighting.Technology`, which should restore live sync functionality for the default project file.

## [0.5.0 Alpha 6](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.6) (March 19, 2019)
* Fixed `rojo init` giving unexpected results by upgrading to `rbx_dom_weak` 1.1.0
* Fixed live server not responding when the Rojo plugin is connected ([#133](https://github.com/rojo-rbx/rojo/issues/133))
* Updated default place file:
    * Improved default properties to be closer to Studio's built-in 'Baseplate' template
    * Added a baseplate to the project file (Thanks, [@AmaranthineCodices](https://github.com/AmaranthineCodices/)!)
* Added more type support to Rojo plugin
* Fixed some cases where the Rojo plugin would leave around objects that it knows should be deleted
* Updated plugin to correctly listen to `Plugin.Unloading` when installing or uninstalling new plugins

## [0.5.0 Alpha 5](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.5) (March 1, 2019)
* Upgraded core dependencies, which improves compatibility for lots of instance types
    * Upgraded from `rbx_tree` 0.2.0 to `rbx_dom_weak` 1.0.0
    * Upgraded from `rbx_xml` 0.2.0 to `rbx_xml` 0.4.0
    * Upgraded from `rbx_binary` 0.2.0 to `rbx_binary` 0.4.0
* Added support for non-primitive types in the Rojo plugin.
    * Types like `Color3` and `CFrame` can now be updated live!
* Fixed plugin assets flashing in on first load ([#121](https://github.com/rojo-rbx/rojo/issues/121))
* Changed Rojo's HTTP server from Rouille to Hyper, which reduced the release size by around a megabyte.
* Added property type inference to projects, which makes specifying services a lot easier ([#130](https://github.com/rojo-rbx/rojo/pull/130))
* Made error messages from invalid and missing files more user-friendly

## [0.5.0 Alpha 4](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.4) (February 8, 2019)
* Added support for nested partitions ([#102](https://github.com/rojo-rbx/rojo/issues/102))
* Added support for 'transmuting' partitions ([#112](https://github.com/rojo-rbx/rojo/issues/112))
* Added support for aliasing filesystem paths ([#105](https://github.com/rojo-rbx/rojo/issues/105))
* Changed Windows builds to statically link the CRT ([#89](https://github.com/rojo-rbx/rojo/issues/89))

## [0.5.0 Alpha 3](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.3) (February 1, 2019)
* Changed default project file name from `roblox-project.json` to `default.project.json` ([#120](https://github.com/rojo-rbx/rojo/pull/120))
    * The old file name will still be supported until 0.5.0 is fully released.
* Added warning when loading project files that don't end in `.project.json`
    * This new extension enables Rojo to distinguish project files from random JSON files, which is necessary to support nested projects.
* Added new (empty) diagnostic page served from the server
* Added better error messages for when a file is missing that's referenced by a Rojo project
* Added support for visualization endpoints returning GraphViz source when Dot is not available
* Fixed an in-memory filesystem regression introduced recently ([#119](https://github.com/rojo-rbx/rojo/pull/119))

## [0.5.0 Alpha 2](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.2) (January 28, 2019)
* Added support for `.model.json` files, compatible with 0.4.x
* Fixed in-memory filesystem not handling out-of-order filesystem change events
* Fixed long-polling error caused by a promise mixup ([#110](https://github.com/rojo-rbx/rojo/issues/110))

## [0.5.0 Alpha 1](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.1) (January 25, 2019)
* Changed plugin UI to be way prettier
    * Thanks to [Reselim](https://github.com/Reselim) for the design!
* Changed plugin error messages to be a little more useful
* Removed unused 'Config' button in plugin UI
* Fixed bug where bad server responses could cause the plugin to be in a bad state
* Upgraded to rbx\_tree, rbx\_xml, and rbx\_binary 0.2.0, which dramatically expands the kinds of properties that Rojo can handle, especially in XML.

## [0.5.0 Alpha 0](https://github.com/rojo-rbx/rojo/releases/tag/v0.5.0-alpha.0) (January 14, 2019)
* "Epiphany" rewrite, in progress since the beginning of time
* New live sync protocol
    * Uses HTTP long polling to reduce request count and improve responsiveness
* New project format
    * Hierarchical, preventing overlapping partitions
* Added `rojo build` command
    * Generates `rbxm`, `rbxmx`, `rbxl`, or `rbxlx` files out of your project
    * Usage: `rojo build <PROJECT> --output <OUTPUT>.rbxm`
* Added `rojo upload` command
    * Generates and uploads a place or model to roblox.com out of your project
    * Usage: `rojo upload <PROJECT> --cookie "<ROBLOSECURITY>" --asset_id <PLACE_ID>`
* New plugin
    * Only one button now, "Connect"
    * New UI to pick server address and port
    * Better error reporting
* Added support for `.csv` files turning into `LocalizationTable` instances
* Added support for `.txt` files turning into `StringValue` instances
* Added debug visualization code to diagnose problems
    * `/visualize/rbx` and `/visualize/imfs` show instance and file state respectively; they require GraphViz to be installed on your machine.
* Added optional place ID restrictions to project files
    * This helps prevent syncing in content to the wrong place
    * Multiple places can be specified, like when building a multi-place game
* Added support for specifying properties on services in project files

## [0.4.13](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.13) (November 12, 2018)
* When `rojo.json` points to a file or directory that does not exist, Rojo now issues a warning instead of throwing an error and exiting

## [0.4.12](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.12) (June 21, 2018)
* Fixed obscure assertion failure when renaming or deleting files ([#78](https://github.com/rojo-rbx/rojo/issues/78))
* Added a `PluginAction` for the sync in command, which should help with some automation scripts ([#80](https://github.com/rojo-rbx/rojo/pull/80))

## [0.4.11](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.11) (June 10, 2018)
* Defensively insert existing instances into RouteMap; should fix most duplication cases when syncing into existing trees.
* Fixed incorrect synchronization from `Plugin:_pull` that would cause polling to create issues
* Fixed incorrect file routes being assigned to `init.lua` and `init.model.json` files
* Untangled route handling-internals slightly

## [0.4.10](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.10) (June 2, 2018)
* Added support for `init.model.json` files, which enable versioning `Tool` instances (among other things) with Rojo. ([#66](https://github.com/rojo-rbx/rojo/issues/66))
* Fixed obscure error when syncing into an invalid service.
* Fixed multiple sync processes occurring when a server ID mismatch is detected.

## [0.4.9](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.9) (May 26, 2018)
* Fixed warning when renaming or removing files that would sometimes corrupt the instance cache ([#72](https://github.com/rojo-rbx/rojo/pull/72))
* JSON models are no longer as strict -- `Children` and `Properties` are now optional.

## [0.4.8](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.8) (May 26, 2018)
* Hotfix to prevent errors from being thrown when objects managed by Rojo are deleted

## [0.4.7](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.7) (May 25, 2018)
* Added icons to the Rojo plugin, made by [@Vorlias](https://github.com/Vorlias)! ([#70](https://github.com/rojo-rbx/rojo/pull/70))
* Server will now issue a warning if no partitions are specified in `rojo serve` ([#40](https://github.com/rojo-rbx/rojo/issues/40))

## [0.4.6](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.6) (May 21, 2018)
* Rojo handles being restarted by Roblox Studio more gracefully ([#67](https://github.com/rojo-rbx/rojo/issues/67))
* Folders should no longer get collapsed when syncing occurs.
* **Significant** robustness improvements with regards to caching.
    * **This should catch all existing script duplication bugs.**
    * If there are any bugs with script duplication or caching in the future, restarting the Rojo server process will fix them for that session.
* Fixed message in plugin not being prefixed with `Rojo: `.

## [0.4.5](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.5) (May 1, 2018)
* Rojo messages are now prefixed with `Rojo: ` to make them stand out in the output more.
* Fixed server to notice file changes *much* more quickly. (200ms vs 1000ms)
* Server now lists name of project when starting up.
* Rojo now throws an error if no project file is found. ([#63](https://github.com/rojo-rbx/rojo/issues/63))
* Fixed multiple sync operations occuring at the same time. ([#61](https://github.com/rojo-rbx/rojo/issues/61))
* Partitions targeting files directly now work as expected. ([#57](https://github.com/rojo-rbx/rojo/issues/57))

## [0.4.4](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.4) (April 7, 2018)
* Fix small regression introduced in 0.4.3

## [0.4.3](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.3) (April 7, 2018)
* Plugin now automatically selects `HttpService` if it determines that HTTP isn't enabled ([#58](https://github.com/rojo-rbx/rojo/pull/58))
* Plugin now has much more robust handling and will wipe all state when the server changes.
    * This should fix issues that would otherwise be solved by restarting Roblox Studio.

## [0.4.2](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.2) (April 4, 2018)
* Fixed final case of duplicated instance insertion, caused by reconciled instances not being inserted into `RouteMap`.
    * The reconciler is still not a perfect solution, especially if script instances get moved around without being destroyed. I don't think this can be fixed before a big refactor.

## [0.4.1](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.1) (April 1, 2018)
* Merged plugin repository into main Rojo repository for easier tracking.
* Improved `RouteMap` object tracking; this should fix some cases of duplicated instances being synced into the tree.

## [0.4.0](https://github.com/rojo-rbx/rojo/releases/tag/v0.4.0) (March 27, 2018)
* Protocol version 1, which shifts more responsibility onto the server
    * This is a **major breaking** change!
    * The server now has a content of 'filter plugins', which transform data at various stages in the pipeline
    * The server now exposes Roblox instance objects instead of file contents, which lines up with how `rojo pack` will work, and paves the way for more robust syncing.
* Added `*.model.json` files, which let you embed small Roblox objects into your Rojo tree.
* Improved error messages in some cases ([#46](https://github.com/rojo-rbx/rojo/issues/46))

## [0.3.2](https://github.com/rojo-rbx/rojo/releases/tag/v0.3.2) (December 20, 2017)
* Fixed `rojo serve` failing to correctly construct an absolute root path when passed as an argument
* Fixed intense CPU usage when running `rojo serve`

## [0.3.1](https://github.com/rojo-rbx/rojo/releases/tag/v0.3.1) (December 14, 2017)
* Improved error reporting when invalid JSON is found in a `rojo.json` project
    * These messages are passed on from Serde

## [0.3.0](https://github.com/rojo-rbx/rojo/releases/tag/v0.3.0) (December 12, 2017)
* Factored out the plugin into a separate repository
* Fixed server when using a file as a partition
    * Previously, trailing slashes were put on the end of a partition even if the read request was an empty string. This broke file reading on Windows when a partition pointed to a file instead of a directory!
* Started running automatic tests on Travis CI (#9)

## [0.2.3](https://github.com/rojo-rbx/rojo/releases/tag/v0.2.3) (December 4, 2017)
* Plugin only release
* Tightened `init` file rules to only match script files
    * Previously, Rojo would sometimes pick up the wrong file when syncing

## [0.2.2](https://github.com/rojo-rbx/rojo/releases/tag/v0.2.2) (December 1, 2017)
* Plugin only release
* Fixed broken reconciliation behavior with `init` files

## [0.2.1](https://github.com/rojo-rbx/rojo/releases/tag/v0.2.1) (December 1, 2017)
* Plugin only release
* Changes default port to 8000

## [0.2.0](https://github.com/rojo-rbx/rojo/releases/tag/v0.2.0) (December 1, 2017)
* Support for `init.lua` like rbxfs and rbxpacker
* More robust syncing with a new reconciler

## [0.1.0](https://github.com/rojo-rbx/rojo/releases/tag/v0.1.0) (November 29, 2017)
* Initial release, functionally very similar to [rbxfs](https://github.com/LPGhatguy/rbxfs)
