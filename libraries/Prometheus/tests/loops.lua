local x = {};
for i = 1, 100 do
    x[i] = i;
end

for i, v in ipairs(x) do
    print("x[" .. i .. "] = " .. v);
end
