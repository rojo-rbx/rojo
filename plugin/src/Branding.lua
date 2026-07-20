local Branding = {
	Name = "Prism",
	Tagline = "Studio automation for Roblox",
	FullLogoAssetId = "rbxassetid://126486056510599",
	IconAssetId = "rbxassetid://132751330860535",
	WarningAssetId = "rbxassetid://104859841226668",
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
