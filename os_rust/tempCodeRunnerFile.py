# sun_rise_down.py
import datetime
import astral
location_XiChong = astral.Location(('XiChong', 'China',  22.484786,114.549965, 'Asia/Shanghai', 0))
# 记录西冲地点，注意先经度后维度 
sunrise=location_XiChong.sunrise(date=datetime.date.today(),local=True)
# 计算今天的日出时间
time_sunrise_new=str(sunrise)
print(time_sunrise_new)
