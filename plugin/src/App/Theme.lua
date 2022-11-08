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

-- Copying hex colors back and forth from design programs is faster
local function hexColor(decimal)
	local red = bit32.band(bit32.rshift(decimal, 16), 2^8 - 1)
	local green = bit32.band(bit32.rshift(decimal, 8), 2^8 - 1)
	local blue = bit32.band(decimal, 2^8 - 1)

	return Color3.fromRGB(red, green, blue)
end

local BRAND_COLOR = hexColor(0xE13835)

local lightTheme = strict("LightTheme", {
	BackgroundColor = hexColor(0xFFFFFF),
	Button = {
		Solid = {
			ActionFillColor = hexColor(0xFFFFFF),
			ActionFillTransparency = 0.8,
			Enabled = {
				TextColor = hexColor(0xFFFFFF),
				BackgroundColor = BRAND_COLOR,
			},
			Disabled = {
				TextColor = hexColor(0xFFFFFF),
				BackgroundColor = BRAND_COLOR,
			},
		},
		Bordered = {
			ActionFillColor = hexColor(0x000000),
			ActionFillTransparency = 0.9,
			Enabled = {
				TextColor = hexColor(0x393939),
				BorderColor = hexColor(0xACACAC),
			},
			Disabled = {
				TextColor = hexColor(0x393939),
				BorderColor = hexColor(0xACACAC),
			},
		},
	},
	Checkbox = {
		Active = {
			IconColor = hexColor(0xFFFFFF),
			BackgroundColor = BRAND_COLOR,
		},
		Inactive = {
			IconColor = hexColor(0xEEEEEE),
			BorderColor = hexColor(0xAFAFAF),
		},
	},
	Dropdown = {
		TextColor = hexColor(0x00000),
		BorderColor = hexColor(0xAFAFAF),
		BackgroundColor = hexColor(0xEEEEEE),
		Open = {
			IconColor = BRAND_COLOR,
		},
		Closed = {
			IconColor = hexColor(0xEEEEEE),
		},
	},
	AddressEntry = {
		TextColor = hexColor(0x000000),
		PlaceholderColor = hexColor(0x8C8C8C)
	},
	BorderedContainer = {
		BorderColor = hexColor(0xCBCBCB),
		BackgroundColor = hexColor(0xEEEEEE),
	},
	Spinner = {
		ForegroundColor = BRAND_COLOR,
		BackgroundColor = hexColor(0xEEEEEE),
	},
	ConnectionDetails = {
		ProjectNameColor = hexColor(0x00000),
		AddressColor = hexColor(0x00000),
		DisconnectColor = BRAND_COLOR,
	},
	Settings = {
		DividerColor = hexColor(0xCBCBCB),
		Navbar = {
			BackButtonColor = hexColor(0x000000),
			TextColor = hexColor(0x000000),
		},
		Setting = {
			NameColor = hexColor(0x000000),
			DescriptionColor = hexColor(0x5F5F5F),
		},
	},
	Header = {
		LogoColor = BRAND_COLOR,
		VersionColor = hexColor(0x727272),
	},
	Notification = {
		InfoColor = hexColor(0x00000),
		CloseColor = BRAND_COLOR,
	},
	ErrorColor = hexColor(0x000000),
	ScrollBarColor = hexColor(0x000000),
})

local darkTheme = strict("DarkTheme", {
	BackgroundColor = hexColor(0x2E2E2E),
	Button = {
		Solid = {
			ActionFillColor = hexColor(0xFFFFFF),
			ActionFillTransparency = 0.8,
			Enabled = {
				TextColor = hexColor(0xFFFFFF),
				BackgroundColor = BRAND_COLOR,
			},
			Disabled = {
				TextColor = hexColor(0xFFFFFF),
				BackgroundColor = BRAND_COLOR,
			},
		},
		Bordered = {
			ActionFillColor = hexColor(0xFFFFFF),
			ActionFillTransparency = 0.9,
			Enabled = {
				TextColor = hexColor(0xDBDBDB),
				BorderColor = hexColor(0x535353),
			},
			Disabled = {
				TextColor = hexColor(0xDBDBDB),
				BorderColor = hexColor(0x535353),
			},
		},
	},
	Checkbox = {
		Active = {
			IconColor = hexColor(0xFFFFFF),
			BackgroundColor = BRAND_COLOR,
		},
		Inactive = {
			IconColor = hexColor(0x484848),
			BorderColor = hexColor(0x5A5A5A),
		},
	},
	Dropdown = {
		TextColor = hexColor(0xFFFFFF),
		BorderColor = hexColor(0x5A5A5A),
		BackgroundColor = hexColor(0x2B2B2B),
		Open = {
			IconColor = BRAND_COLOR,
		},
		Closed = {
			IconColor = hexColor(0x484848),
		},
	},
	AddressEntry = {
		TextColor = hexColor(0xFFFFFF),
		PlaceholderColor = hexColor(0x8B8B8B)
	},
	BorderedContainer = {
		BorderColor = hexColor(0x535353),
		BackgroundColor = hexColor(0x2B2B2B),
	},
	Spinner = {
		ForegroundColor = BRAND_COLOR,
		BackgroundColor = hexColor(0x2B2B2B),
	},
	ConnectionDetails = {
		ProjectNameColor = hexColor(0xFFFFFF),
		AddressColor = hexColor(0xFFFFFF),
		DisconnectColor = hexColor(0xFFFFFF),
	},
	Settings = {
		DividerColor = hexColor(0x535353),
		Navbar = {
			BackButtonColor = hexColor(0xFFFFFF),
			TextColor = hexColor(0xFFFFFF),
		},
		Setting = {
			NameColor = hexColor(0xFFFFFF),
			DescriptionColor = hexColor(0xD3D3D3),
		},
	},
	Header = {
		LogoColor = BRAND_COLOR,
		VersionColor = hexColor(0xD3D3D3)
	},
	Notification = {
		InfoColor = hexColor(0xFFFFFF),
		CloseColor = hexColor(0xFFFFFF),
	},
	ErrorColor = hexColor(0xFFFFFF),
	ScrollBarColor = hexColor(0xFFFFFF),
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
