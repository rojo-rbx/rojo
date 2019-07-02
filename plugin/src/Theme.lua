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

setmetatable(Theme, {
	__index = function(_, key)
		error(("%s is not a valid member of Theme"):format(key), 2)
	end
})

return Theme