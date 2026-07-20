local Branding = {
	Name = "Prism",
	Tagline = "Studio automation for Roblox",
	FullLogoAssetId = "rbxassetid://78583834238790",
	IconAssetId = "rbxassetid://84145747248222",
	WarningAssetId = "rbxassetid://130567248372259",
	-- Retain the original field for consumers that already use it as the compact icon.
	AssetId = "rbxassetid://84145747248222",
	DockWidgetTitlePrefix = "Prism ",
	Compatibility = {
		DockWidgetIdPrefix = "Rojo ",
		ToolbarButtonId = "Rojo",
		ToggleActionId = "RojoConnection",
		ConnectActionId = "RojoConnect",
		DisconnectActionId = "RojoDisconnect",
	},
}

return table.freeze(Branding)
