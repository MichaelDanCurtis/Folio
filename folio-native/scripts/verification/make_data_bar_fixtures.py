#!/usr/bin/env python3
"""Reproducible XlsxWriter import corpus; not captured Excel behavior."""
from pathlib import Path
import xlsxwriter
p=Path(__file__).resolve().parents[2]/'rust/tests/fixtures/data-bars.xlsx'
w=xlsxwriter.Workbook(p)
s=w.add_worksheet('Bars')
s.right_to_left()
for col in range(6):
    s.write_column(0,col,[-20,0,40,80])
s.write('H1',-40);s.write('H2',100)
s.conditional_format('A1:A4', {'type':'data_bar','bar_color':'#336699'})
s.conditional_format('B1:B4', {'type':'data_bar','data_bar_2010':True,'bar_color':'#63BE7B','bar_solid':True,'bar_negative_color':'#C00000','bar_negative_border_color':'#800000','bar_axis_color':'#333333','bar_only':True})
s.conditional_format('C1:C4', {'type':'data_bar','data_bar_2010':True,'bar_direction':'right','bar_axis_position':'middle'})
s.conditional_format('D1:D4', {'type':'data_bar','data_bar_2010':True,'bar_axis_position':'none','bar_no_border':True,'bar_negative_color_same':True})
s.conditional_format('E1:E4', {'type':'data_bar','data_bar_2010':True,'min_type':'formula','min_value':'=$H$1','max_type':'formula','max_value':'=$H$2'})
s.conditional_format('F1:F4', {'type':'data_bar','data_bar_2010':True,'min_type':'percentile','min_value':25,'max_type':'percentile','max_value':75,'bar_direction':'left'})
w.close()
print(p)
