-- Print the fibonacci sequence
local function fibonacci(max)
    local a, b = 0, 1
    while a < max do
        print(a)
        a, b = b, a + b
    end
end

fibonacci(1000)