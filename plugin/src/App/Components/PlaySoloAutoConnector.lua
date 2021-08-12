local RunService = game:GetService("RunService")

local Rojo = script:FindFirstAncestor("Rojo")

local Roact = require(Rojo.Roact)

local e = Roact.createElement

local PlaySoloAutoConnector = Roact.Component:extend("PlaySoloAutoConnector")

function PlaySoloAutoConnector:render()
	return nil
end

function PlaySoloAutoConnector:didMount()
	local settings = self.props.settings

	if RunService:IsServer() and RunService:IsRunning() and settings:get("playSoloAutoConnect") then
		local connectionId = workspace:GetAttribute("__RojoConnectionId")

		if connectionId then
			local activeConnections = settings:get("activeConnections", true)
			local activeConnection
			for _, connection in ipairs(activeConnections) do
				if connection.connectionId == connectionId then
					activeConnection = connection
					break
				end
			end

			if activeConnection then
				self.props.onConnect(activeConnection.host, activeConnection.port, activeConnection.sessionId)
			end
		end
	end
end

return PlaySoloAutoConnector
