--[[
	Theming system taking advantage of Roact's new context API.
	Doesn't use colors provided by Studio and instead just branches on theme
	name. This isn't exactly best practice.
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

local Rojo = script:FindFirstAncestor("Rojo")
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Log = require(Packages.Log)

local strict = require(script.Parent.Parent.strict)

local BRAND_COLOR = Color3.fromHex("E13835")

local lightTheme = strict("LightTheme", {
	BackgroundColor = Color3.fromHex("FFFFFF"),
	Button = {
		Solid = {
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
			ActionFillColor = Color3.fromHex("000000"),
			ActionFillTransparency = 0.9,
			Enabled = {
				TextColor = Color3.fromHex("393939"),
				BorderColor = Color3.fromHex("ACACAC"),
			},
			Disabled = {
				TextColor = Color3.fromHex("393939"),
				BorderColor = Color3.fromHex("ACACAC"),
			},
		},
	},
	Checkbox = {
		Active = {
			IconColor = Color3.fromHex("FFFFFF"),
			BackgroundColor = BRAND_COLOR,
		},
		Inactive = {
			IconColor = Color3.fromHex("EEEEEE"),
			BorderColor = Color3.fromHex("AFAFAF"),
		},
	},
	Dropdown = {
		TextColor = Color3.fromHex("000000"),
		BorderColor = Color3.fromHex("AFAFAF"),
		BackgroundColor = Color3.fromHex("EEEEEE"),
		Open = {
			IconColor = BRAND_COLOR,
		},
		Closed = {
			IconColor = Color3.fromHex("EEEEEE"),
		},
	},
	TextInput = {
		Enabled = {
			TextColor = Color3.fromHex("000000"),
			PlaceholderColor = Color3.fromHex("8C8C8C"),
			BorderColor = Color3.fromHex("ACACAC"),
		},
		Disabled = {
			TextColor = Color3.fromHex("393939"),
			PlaceholderColor = Color3.fromHex("8C8C8C"),
			BorderColor = Color3.fromHex("AFAFAF"),
		},
		ActionFillColor = Color3.fromHex("000000"),
		ActionFillTransparency = 0.9,
	},
	AddressEntry = {
		TextColor = Color3.fromHex("000000"),
		PlaceholderColor = Color3.fromHex("8C8C8C"),
	},
	BorderedContainer = {
		BorderColor = Color3.fromHex("CBCBCB"),
		BackgroundColor = Color3.fromHex("EEEEEE"),
	},
	Spinner = {
		ForegroundColor = BRAND_COLOR,
		BackgroundColor = Color3.fromHex("EEEEEE"),
	},
	Diff = {
		Add = Color3.fromHex("baffbd"),
		Remove = Color3.fromHex("ffbdba"),
		Edit = Color3.fromHex("bacdff"),
		Row = Color3.fromHex("000000"),
		Warning = Color3.fromHex("FF8E3C"),
	},
	ConnectionDetails = {
		ProjectNameColor = Color3.fromHex("000000"),
		AddressColor = Color3.fromHex("000000"),
		DisconnectColor = BRAND_COLOR,
	},
	Settings = {
		DividerColor = Color3.fromHex("CBCBCB"),
		Navbar = {
			BackButtonColor = Color3.fromHex("000000"),
			TextColor = Color3.fromHex("000000"),
		},
		Setting = {
			NameColor = Color3.fromHex("000000"),
			DescriptionColor = Color3.fromHex("5F5F5F"),
		},
	},
	Header = {
		LogoColor = BRAND_COLOR,
		VersionColor = Color3.fromHex("727272"),
	},
	Notification = {
		InfoColor = Color3.fromHex("000000"),
		CloseColor = BRAND_COLOR,
	},
	ErrorColor = Color3.fromHex("000000"),
	ScrollBarColor = Color3.fromHex("000000"),
})

local darkTheme = strict("DarkTheme", {
	BackgroundColor = Color3.fromHex("2E2E2E"),
	Button = {
		Solid = {
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
			ActionFillColor = Color3.fromHex("FFFFFF"),
			ActionFillTransparency = 0.9,
			Enabled = {
				TextColor = Color3.fromHex("DBDBDB"),
				BorderColor = Color3.fromHex("535353"),
			},
			Disabled = {
				TextColor = Color3.fromHex("DBDBDB"),
				BorderColor = Color3.fromHex("535353"),
			},
		},
	},
	Checkbox = {
		Active = {
			IconColor = Color3.fromHex("FFFFFF"),
			BackgroundColor = BRAND_COLOR,
		},
		Inactive = {
			IconColor = Color3.fromHex("484848"),
			BorderColor = Color3.fromHex("5A5A5A"),
		},
	},
	Dropdown = {
		TextColor = Color3.fromHex("FFFFFF"),
		BorderColor = Color3.fromHex("5A5A5A"),
		BackgroundColor = Color3.fromHex("2B2B2B"),
		Open = {
			IconColor = BRAND_COLOR,
		},
		Closed = {
			IconColor = Color3.fromHex("484848"),
		},
	},
	TextInput = {
		Enabled = {
			TextColor = Color3.fromHex("FFFFFF"),
			PlaceholderColor = Color3.fromHex("8B8B8B"),
			BorderColor = Color3.fromHex("535353"),
		},
		Disabled = {
			TextColor = Color3.fromHex("484848"),
			PlaceholderColor = Color3.fromHex("8B8B8B"),
			BorderColor = Color3.fromHex("5A5A5A"),
		},
		ActionFillColor = Color3.fromHex("FFFFFF"),
		ActionFillTransparency = 0.9,
	},
	AddressEntry = {
		TextColor = Color3.fromHex("FFFFFF"),
		PlaceholderColor = Color3.fromHex("8B8B8B"),
	},
	BorderedContainer = {
		BorderColor = Color3.fromHex("535353"),
		BackgroundColor = Color3.fromHex("2B2B2B"),
	},
	Spinner = {
		ForegroundColor = BRAND_COLOR,
		BackgroundColor = Color3.fromHex("2B2B2B"),
	},
	Diff = {
		Add = Color3.fromHex("273732"),
		Remove = Color3.fromHex("3F2D32"),
		Edit = Color3.fromHex("193345"),
		Row = Color3.fromHex("FFFFFF"),
		Warning = Color3.fromHex("FF8E3C"),
	},
	ConnectionDetails = {
		ProjectNameColor = Color3.fromHex("FFFFFF"),
		AddressColor = Color3.fromHex("FFFFFF"),
		DisconnectColor = Color3.fromHex("FFFFFF"),
	},
	Settings = {
		DividerColor = Color3.fromHex("535353"),
		Navbar = {
			BackButtonColor = Color3.fromHex("FFFFFF"),
			TextColor = Color3.fromHex("FFFFFF"),
		},
		Setting = {
			NameColor = Color3.fromHex("FFFFFF"),
			DescriptionColor = Color3.fromHex("D3D3D3"),
		},
	},
	Header = {
		LogoColor = BRAND_COLOR,
		VersionColor = Color3.fromHex("D3D3D3"),
	},
	Notification = {
		InfoColor = Color3.fromHex("FFFFFF"),
		CloseColor = Color3.fromHex("FFFFFF"),
	},
	ErrorColor = Color3.fromHex("FFFFFF"),
	ScrollBarColor = Color3.fromHex("FFFFFF"),
})

local Context = Roact.createContext(lightTheme)

local StudioProvider = Roact.Component:extend("StudioProvider")

-- Pull the current theme from Roblox Studio and update state with it.
function StudioProvider:updateTheme()
	local studioTheme = getStudio().Theme

	if studioTheme.Name == "Light" then
		self:setState({
			theme = lightTheme,
		})
	elseif studioTheme.Name == "Dark" then
		self:setState({
			theme = darkTheme,
		})
	else
		Log.warn("Unexpected theme '{}'' -- falling back to light theme!", studioTheme.Name)

		self:setState({
			theme = lightTheme,
		})
	end
end

function StudioProvider:init()
	self:updateTheme()
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
