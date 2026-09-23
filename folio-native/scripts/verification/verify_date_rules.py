#!/usr/bin/env python3
"""Independent OOXML date-rule fixtures/readers; never claim Excel execution."""
import argparse, json
from pathlib import Path
from datetime import datetime
import openpyxl
from openpyxl.formatting.rule import Rule
from openpyxl.styles import PatternFill
from openpyxl.styles.differential import DifferentialStyle
from openpyxl.utils.datetime import CALENDAR_MAC_1904
PERIODS=['yesterday','today','tomorrow','last7Days','lastWeek','thisWeek','nextWeek','lastMonth','thisMonth','nextMonth']
def fixtures(folder):
    folder.mkdir(parents=True,exist_ok=True)
    for calendar in [1900,1904]:
        wb=openpyxl.Workbook()
        if calendar==1904: wb.epoch=CALENDAR_MAC_1904
        ws=wb.active;ws.title='External dates'
        for row,date in enumerate([datetime(2024,12,31),datetime(2025,1,1,23,59,59),datetime(2025,1,2)],1):ws.cell(row,1,date)
        ws['B1']=1;ws['C1']='=NOW()'
        for i,period in enumerate(PERIODS):
            # Omitted formulas exercise metadata-only rules from independent producers.
            formula=['AND(A1=TODAY(),$B$1=1)'] if period=='today' else []
            rule=Rule(type='timePeriod',timePeriod=period,formula=formula,stopIfTrue=True,
                dxf=DifferentialStyle(fill=PatternFill('solid',fgColor='FFAABBCC')))
            ws.conditional_formatting.add('A1:A3',rule)
        path=folder/f'date-periods-{calendar}.xlsx';wb.save(path)
    return {'producer':'openpyxl','calendars':[1900,1904]}
def rules(book):
    return [(str(group.sqref),r) for group in book.active.conditional_formatting for r in book.active.conditional_formatting[group]]
def verify(core,ui):
    files=[]
    for name,anchor in [('date-periods.xlsx','A1'),('date-periods-shifted.xlsx','A2'),('date-periods-recovered.xlsx','A2')]:
        book=openpyxl.load_workbook(core/name);rs=rules(book)
        assert sorted(r.timePeriod for _,r in rs)==sorted(PERIODS)
        assert sorted(r.priority for _,r in rs)==list(range(1,11))
        for area,r in rs:
            assert area==('A1:A4 C1:C4' if anchor=='A1' else 'A2:A5 C2:C5')
            assert r.type=='timePeriod' and len(r.formula)==1 and f'ISNUMBER({anchor})' in r.formula[0]
            assert r.dxf.fill.fgColor.rgb=='FFABCDEF' and r.dxf.font.b
        cached=openpyxl.load_workbook(core/name,data_only=True).active['B1' if anchor=='A1' else 'B2'].value
        assert cached==(45658.625 if name=='date-periods-recovered.xlsx' else 45658.5)
        files.append(name)
    for cal in [1900,1904]:
        name=f'date-independent-{cal}.xlsx';book=openpyxl.load_workbook(core/name)
        assert (book.epoch==CALENDAR_MAC_1904)==(cal==1904)
        rs=rules(book);assert len(rs)==10
        for _,r in rs:
            assert r.timePeriod in PERIODS
            assert r.formula==(['AND(A1=TODAY(),$B$1=1)'] if r.timePeriod=='today' else [])
        assert book.active['A2'].value==datetime(2025,1,1,23,59,59)
        assert openpyxl.load_workbook(core/name,data_only=True).active['C1'].value==(44196.5 if cal==1904 else 45658.5)
        files.append(name)
        name=f'date-independent-shifted-{cal}.xlsx';book=openpyxl.load_workbook(core/name)
        assert (book.epoch==CALENDAR_MAC_1904)==(cal==1904)
        for area,r in rules(book):
            assert area=='A2:A4'
            assert r.formula==(['AND(A2=TODAY(),$B$2=1)'] if r.timePeriod=='today' else [])
        assert book.active['C2'].value=='=NOW()'
        assert openpyxl.load_workbook(core/name,data_only=True).active['C2'].value==(44196.5 if cal==1904 else 45658.5)
        files.append(name)
    if ui:
        name='dates.xlsx';book=openpyxl.load_workbook(ui/name);rs=rules(book)
        assert len(rs)==1 and rs[0][0]=='A1:A1048576' and rs[0][1].timePeriod=='tomorrow'
        assert book.active['A2'].value=='=TODAY()' and book.active['B1'].value=='=NOW()'
        files.append(name)
    return {'passed':True,'reader':'openpyxl','files':files,'nativeExcelComparison':False}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('core',type=Path,nargs='?');p.add_argument('--ui',type=Path);p.add_argument('--fixtures',type=Path);a=p.parse_args()
    print(json.dumps(fixtures(a.fixtures) if a.fixtures else verify(a.core,a.ui),indent=2))
