local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Packages = Rojo.Packages

local Log = require(Packages.Log)

local params = Instance.new("GetTextBoundsParams")

local function getTextBoundsAsync(
	text: string,
	font: Font,
	textSize: number,
	width: number,
	richText: boolean?
): Vector2
	if type(text) ~= "string" then
		Log.warn(`Invalid text. Expected string, received {type(text)} instead`)
		return Vector2.zero
	end
	if #text >= 200_000 then
		Log.warn(`Invalid text. Exceeds the 199,999 character limit`)
		return Vector2.zero
	end

	params.Text = text
	params.Font = font
	params.Size = textSize
	params.Width = width
	params.RichText = not not richText

	local success, bounds = pcall(TextService.GetTextBoundsAsync, TextService, params)
	if not success then
		Log.warn(`Failed to get text bounds: {bounds}`)
		return Vector2.zero
	end

	return bounds
end

return getTextBoundsAsync
