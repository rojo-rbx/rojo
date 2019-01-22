local Theme = {
	ButtonFont = Enum.Font.SourceSansSemibold,
	TitleFont = Enum.Font.SourceSansSemibold,
	MainFont = Enum.Font.SourceSans,
	AccentColor = Color3.fromRGB(136, 0, 27),
	PrimaryColor = Color3.fromRGB(20, 20, 20),
	SecondaryColor = Color3.fromRGB(240, 240, 240),
}

setmetatable(Theme, {
	__index = function(_, key)
		error(("%s is not a valid member of Theme"):format(key), 2)
	end
})

return Theme