local Theme = {
	ButtonFont = Enum.Font.GothamSemibold,
	InputFont = Enum.Font.Code,
	TitleFont = Enum.Font.GothamBold,
	MainFont = Enum.Font.Gotham,

	AccentColor = Color3.fromRGB(225, 56, 53),
	AccentLightColor = Color3.fromRGB(255, 146, 145),
	PrimaryColor = Color3.fromRGB(64, 64, 64),
	SecondaryColor = Color3.fromRGB(235, 235, 235),
	LightTextColor = Color3.fromRGB(160, 160, 160),
}

--[[ local DarkTheme = {

	Little unsure on how to change the color of the background on the plugin for dark mode...

	ButtonFont = Enum.Font.GothamSemibold,
	InputFont = Enum.Font.Code,
	TitleFont = Enum.Font.GothamBold,
	MainFont = Enum.Font.Gotham,
	
	AccentColor = Color3.fromRGB(89, 88, 88),
	AccentLightColor = Color3.fromRGB(255, 146, 145),
	PrimaryColor = Color3.fromRGB(53, 53, 53),
	SecondaryColor = Color3.fromRGB(103, 103, 103),
	LightTextColor = Color3.fromRGB(103, 103, 103)
} ]]--

if Enum.UITheme == Enum.UITheme.Light then
	setmetatable(Theme, {
	__index = function(_, key)
		error(("%s is not a valid member of Theme"):format(key), 2)
	end
	})
	return Theme
--[[ else
	setmetatable(DarkTheme, {
		__index = function(_, key)
			error(("%s is not a valid member of Theme"):format(key), 2)
		end
	})
	return DarkTheme
]]--
end 
