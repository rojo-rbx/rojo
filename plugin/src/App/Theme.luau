--[[
	Theming system provided through Roact's context.
	Uses Studio colors when possible.
]]

-- Studio does not exist outside Roblox Studio, so we'll lazily initialize it
-- when possible.
local _Studio
local function getStudio()
	if _Studio == nil then
		_Studio = settings():GetService("Studio")
	end

	return _Studio
end

local ContentProvider = game:GetService("ContentProvider")

local Rojo = script:FindFirstAncestor("Rojo")
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local strict = require(script.Parent.Parent.strict)

local BRAND_COLOR = Color3.fromHex("E13835")

local Context = Roact.createContext({})

local StudioProvider = Roact.Component:extend("StudioProvider")

-- Pull the current theme from Roblox Studio and update state with it.
function StudioProvider:updateTheme()
	local studioTheme = getStudio().Theme

	local isDark = studioTheme.Name == "Dark"

	local theme = strict(studioTheme.Name .. "Theme", {
		Font = {
			Main = Font.new("rbxasset://fonts/families/Montserrat.json", Enum.FontWeight.Medium, Enum.FontStyle.Normal),
			Bold = Font.new("rbxasset://fonts/families/Montserrat.json", Enum.FontWeight.Bold, Enum.FontStyle.Normal),
			Thin = Font.new(
				"rbxasset://fonts/families/Montserrat.json",
				Enum.FontWeight.Regular,
				Enum.FontStyle.Normal
			),
			Code = Font.new(
				"rbxasset://fonts/families/Inconsolata.json",
				Enum.FontWeight.Regular,
				Enum.FontStyle.Normal
			),
		},
		TextSize = {
			Body = 15,
			Small = 13,
			Medium = 16,
			Large = 18,
			Code = 16,
		},
		BrandColor = BRAND_COLOR,
		BackgroundColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.MainBackground),
		TextColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.MainText),
		SubTextColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.SubText),
		Button = {
			Solid = {
				-- Solid uses brand theming, not Studio theming.
				ActionFillColor = Color3.fromHex("FFFFFF"),
				ActionFillTransparency = 0.8,
				Enabled = {
					TextColor = Color3.fromHex("FFFFFF"),
					BackgroundColor = BRAND_COLOR,
				},
				Disabled = {
					TextColor = Color3.fromHex("FFFFFF"),
					BackgroundColor = BRAND_COLOR,
				},
			},
			Bordered = {
				ActionFillColor = studioTheme:GetColor(
					Enum.StudioStyleGuideColor.ButtonText,
					Enum.StudioStyleGuideModifier.Selected
				),
				ActionFillTransparency = 0.9,
				Enabled = {
					TextColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.ButtonText),
					BorderColor = studioTheme:GetColor(
						Enum.StudioStyleGuideColor.CheckedFieldBorder,
						Enum.StudioStyleGuideModifier.Disabled
					),
				},
				Disabled = {
					TextColor = studioTheme:GetColor(
						Enum.StudioStyleGuideColor.ButtonText,
						Enum.StudioStyleGuideModifier.Disabled
					),
					BorderColor = studioTheme:GetColor(
						Enum.StudioStyleGuideColor.CheckedFieldBorder,
						Enum.StudioStyleGuideModifier.Disabled
					),
				},
			},
		},
		Checkbox = {
			Active = {
				-- Active checkboxes use brand theming, not Studio theming.
				IconColor = Color3.fromHex("FFFFFF"),
				BackgroundColor = BRAND_COLOR,
			},
			Inactive = {
				IconColor = studioTheme:GetColor(
					Enum.StudioStyleGuideColor.CheckedFieldIndicator,
					Enum.StudioStyleGuideModifier.Disabled
				),
				BorderColor = studioTheme:GetColor(
					Enum.StudioStyleGuideColor.CheckedFieldBorder,
					Enum.StudioStyleGuideModifier.Disabled
				),
			},
		},
		Dropdown = {
			TextColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.ButtonText),
			BorderColor = studioTheme:GetColor(
				Enum.StudioStyleGuideColor.CheckedFieldBorder,
				Enum.StudioStyleGuideModifier.Disabled
			),
			BackgroundColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.MainBackground),
			IconColor = studioTheme:GetColor(
				Enum.StudioStyleGuideColor.CheckedFieldIndicator,
				Enum.StudioStyleGuideModifier.Disabled
			),
		},
		TextInput = {
			Enabled = {
				TextColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
				PlaceholderColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.SubText),
				BorderColor = studioTheme:GetColor(
					Enum.StudioStyleGuideColor.CheckedFieldBorder,
					Enum.StudioStyleGuideModifier.Disabled
				),
			},
			Disabled = {
				TextColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.MainText),
				PlaceholderColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.SubText),
				BorderColor = studioTheme:GetColor(
					Enum.StudioStyleGuideColor.CheckedFieldBorder,
					Enum.StudioStyleGuideModifier.Disabled
				),
			},
			ActionFillColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
			ActionFillTransparency = 0.9,
		},
		AddressEntry = {
			TextColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
			PlaceholderColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.SubText),
		},
		BorderedContainer = {
			BorderColor = studioTheme:GetColor(
				Enum.StudioStyleGuideColor.CheckedFieldBorder,
				Enum.StudioStyleGuideModifier.Disabled
			),
			BackgroundColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.InputFieldBackground),
		},
		Spinner = {
			ForegroundColor = BRAND_COLOR,
			BackgroundColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.InputFieldBackground),
		},
		Diff = {
			-- Very bright different colors in case some places were not updated to use
			-- the new background diff colors.
			Add = Color3.fromRGB(255, 0, 255),
			Remove = Color3.fromRGB(255, 0, 255),
			Edit = Color3.fromRGB(255, 0, 255),

			Row = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
			Warning = studioTheme:GetColor(Enum.StudioStyleGuideColor.WarningText),

			Background = {
				-- Studio doesn't have good colors since their diffs use backgrounds, not text
				Add = if isDark then Color3.fromRGB(143, 227, 154) else Color3.fromRGB(41, 164, 45),
				Remove = if isDark then Color3.fromRGB(242, 125, 125) else Color3.fromRGB(150, 29, 29),
				Edit = if isDark then Color3.fromRGB(120, 154, 248) else Color3.fromRGB(0, 70, 160),
				Remain = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
			},

			Text = {
				Add = if isDark then Color3.new(0, 0, 0) else Color3.new(1, 1, 1),
				Remove = if isDark then Color3.new(0, 0, 0) else Color3.new(1, 1, 1),
				Edit = if isDark then Color3.new(0, 0, 0) else Color3.new(1, 1, 1),
				Remain = studioTheme:GetColor(Enum.StudioStyleGuideColor.MainText),
			},
		},
		ConnectionDetails = {
			ProjectNameColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
			AddressColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
			DisconnectColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
		},
		Settings = {
			DividerColor = studioTheme:GetColor(
				Enum.StudioStyleGuideColor.CheckedFieldBorder,
				Enum.StudioStyleGuideModifier.Disabled
			),
			Navbar = {
				BackButtonColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
				TextColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
			},
			Setting = {
				NameColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
				DescriptionColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.MainText),
				UnstableColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.WarningText),
				DebugColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.InfoText),
			},
		},
		Header = {
			LogoColor = BRAND_COLOR,
			VersionColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.MainText),
		},
		Notification = {
			InfoColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
			CloseColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
		},
		ErrorColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
		ScrollBarColor = studioTheme:GetColor(Enum.StudioStyleGuideColor.BrightText),
	})

	self:setState({
		theme = theme,
	})
end

function StudioProvider:init()
	self:updateTheme()

	-- Preload the Fonts so that getTextBoundsAsync won't yield
	local fontAssetIds = {}
	for _, font in self.state.theme.Font do
		table.insert(fontAssetIds, font.Family)
	end
	pcall(ContentProvider.PreloadAsync, ContentProvider, fontAssetIds)
end

function StudioProvider:render()
	return Roact.createElement(Context.Provider, {
		value = self.state.theme,
	}, self.props[Roact.Children])
end

function StudioProvider:didMount()
	self.connection = getStudio().ThemeChanged:Connect(function()
		self:updateTheme()
	end)
end

function StudioProvider:willUnmount()
	self.connection:Disconnect()
end

local function with(callback)
	return Roact.createElement(Context.Consumer, {
		render = callback,
	})
end

return {
	StudioProvider = StudioProvider,
	Consumer = Context.Consumer,
	with = with,
}
