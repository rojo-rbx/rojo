local Theme = {
	ButtonFont = Enum.Font.GothamSemibold,
	InputFont = Enum.Font.Gotham,
	TitleFont = Enum.Font.GothamBold,
	MainFont = Enum.Font.Gotham,

	AccentColor = Color3.fromRGB(136, 0, 27),
	PrimaryColor = Color3.fromRGB(20, 20, 20),
	SecondaryColor = Color3.fromRGB(235, 235, 235),
	LightTextColor = Color3.fromRGB(140, 140, 140),
}

setmetatable(Theme, {
	__index = function(_, key)
		error(("%s is not a valid member of Theme"):format(key), 2)
	end
})

return Theme