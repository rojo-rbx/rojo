--[[
	Prism's theme is provided through Roact context. The fixed dark palette keeps
	the plugin recognizable and readable in either Studio theme while retaining
	the existing Theme API used by the upstream-compatible UI components.
]]

-- Studio does not exist outside Roblox Studio, so we'll lazily initialize it.
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

local Tokens = table.freeze({
	PanelBackground = Color3.fromHex("080D18"),
	CardBackground = Color3.fromHex("101827"),
	ElevatedCardBackground = Color3.fromHex("172238"),
	Border = Color3.fromHex("33415C"),
	PrimaryText = Color3.fromHex("F3F7FF"),
	SecondaryText = Color3.fromHex("C4CDDF"),
	MutedText = Color3.fromHex("8793AA"),
	PrimaryAccent = Color3.fromHex("62DDF2"),
	Success = Color3.fromHex("54D9AA"),
	Warning = Color3.fromHex("F1B85B"),
	Danger = Color3.fromHex("FF7185"),
	ButtonHover = Color3.fromHex("FFFFFF"),
	ButtonPressed = Color3.fromHex("D8E5FF"),
	InputBackground = Color3.fromHex("0C1423"),
	InputBorder = Color3.fromHex("40506D"),
	OverlayDarkness = 0.68,
	CornerRadius = 8,
})

local Context = Roact.createContext({})

local StudioProvider = Roact.Component:extend("StudioProvider")

function StudioProvider:updateTheme()
	local theme = strict("PrismTheme", {
		Tokens = Tokens,
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
		BrandColor = Tokens.PrimaryAccent,
		BackgroundColor = Tokens.PanelBackground,
		TextColor = Tokens.PrimaryText,
		SubTextColor = Tokens.SecondaryText,
		Button = {
			Solid = {
				HasBackground = true,
				HasBorder = false,
				PressedColor = Tokens.ButtonPressed,
				ActionFillColor = Tokens.ButtonHover,
				ActionFillTransparency = 0.86,
				Enabled = {
					TextColor = Tokens.PanelBackground,
					BackgroundColor = Tokens.PrimaryAccent,
				},
				Disabled = {
					TextColor = Tokens.MutedText,
					BackgroundColor = Tokens.Border,
				},
			},
			Bordered = {
				HasBackground = false,
				HasBorder = true,
				PressedColor = Tokens.ButtonPressed,
				ActionFillColor = Tokens.ButtonHover,
				ActionFillTransparency = 0.91,
				Enabled = {
					TextColor = Tokens.SecondaryText,
					BorderColor = Tokens.Border,
				},
				Disabled = {
					TextColor = Tokens.MutedText,
					BorderColor = Tokens.InputBorder,
				},
			},
			Danger = {
				HasBackground = false,
				HasBorder = true,
				PressedColor = Tokens.Danger,
				ActionFillColor = Tokens.Danger,
				ActionFillTransparency = 0.91,
				Enabled = {
					TextColor = Tokens.Danger,
					BorderColor = Tokens.Danger,
				},
				Disabled = {
					TextColor = Tokens.MutedText,
					BorderColor = Tokens.Border,
				},
			},
		},
		Checkbox = {
			Active = {
				IconColor = Tokens.PanelBackground,
				BackgroundColor = Tokens.PrimaryAccent,
			},
			Inactive = {
				IconColor = Tokens.MutedText,
				BorderColor = Tokens.InputBorder,
			},
		},
		Dropdown = {
			TextColor = Tokens.PrimaryText,
			BorderColor = Tokens.InputBorder,
			BackgroundColor = Tokens.ElevatedCardBackground,
			IconColor = Tokens.SecondaryText,
		},
		TextInput = {
			Enabled = {
				TextColor = Tokens.PrimaryText,
				PlaceholderColor = Tokens.MutedText,
				BorderColor = Tokens.InputBorder,
			},
			Disabled = {
				TextColor = Tokens.MutedText,
				PlaceholderColor = Tokens.MutedText,
				BorderColor = Tokens.Border,
			},
			ActionFillColor = Tokens.ButtonHover,
			ActionFillTransparency = 0.94,
			FocusBorderColor = Tokens.PrimaryAccent,
		},
		AddressEntry = {
			TextColor = Tokens.PrimaryText,
			PlaceholderColor = Tokens.MutedText,
			LabelColor = Tokens.MutedText,
			FocusBorderColor = Tokens.PrimaryAccent,
		},
		BorderedContainer = {
			BorderColor = Tokens.Border,
			BorderedColor = Tokens.ElevatedCardBackground,
			BackgroundColor = Tokens.CardBackground,
		},
		Spinner = {
			ForegroundColor = Tokens.PrimaryAccent,
			BackgroundColor = Tokens.Border,
		},
		Diff = {
			Add = Tokens.Success,
			Remove = Tokens.Danger,
			Edit = Color3.fromHex("7F9DFF"),
			Row = Tokens.PrimaryText,
			Warning = Tokens.Warning,
			Background = {
				Add = Color3.fromHex("8DE4B9"),
				Remove = Color3.fromHex("FF9AA8"),
				Edit = Color3.fromHex("9EB3FF"),
				Remain = Tokens.PrimaryText,
			},
			Text = {
				Add = Tokens.PanelBackground,
				Remove = Tokens.PanelBackground,
				Edit = Tokens.PanelBackground,
				Remain = Tokens.PrimaryText,
			},
		},
		ConnectionDetails = {
			ProjectNameColor = Tokens.PrimaryText,
			AddressColor = Tokens.SecondaryText,
			DisconnectColor = Tokens.Danger,
		},
		Settings = {
			DividerColor = Tokens.Border,
			Navbar = {
				BackButtonColor = Tokens.SecondaryText,
				TextColor = Tokens.PrimaryText,
			},
			Setting = {
				NameColor = Tokens.PrimaryText,
				DescriptionColor = Tokens.SecondaryText,
				UnstableColor = Tokens.Warning,
				DebugColor = Tokens.PrimaryAccent,
			},
		},
		Header = {
			LogoColor = Color3.new(1, 1, 1),
			VersionColor = Tokens.MutedText,
		},
		Notification = {
			InfoColor = Tokens.PrimaryText,
			CloseColor = Tokens.SecondaryText,
		},
		ErrorColor = Tokens.PrimaryText,
		ScrollBarColor = Tokens.SecondaryText,
	})

	self:setState({
		theme = theme,
	})
end

function StudioProvider:init()
	self:updateTheme()

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
	Tokens = Tokens,
	StudioProvider = StudioProvider,
	Consumer = Context.Consumer,
	with = with,
}
