"""Independent serial-date expectations: Gregorian dates plus Excel's 1900 convention.

These modeled expectations are not results from running Microsoft Excel.
"""
from datetime import date, timedelta
from pathlib import Path
import json
from openpyxl import Workbook

root = Path(__file__).resolve().parents[2] / 'test/fixtures'
book = Workbook(); sheet = book.active; sheet.title = 'Early dates'
sheet.append(['Serial date', 'Day', 'Month', 'Year', 'Weekday', 'DATE roundtrip',
              'DATEVALUE', 'TEXT', 'One month later', 'Month end'])
expected = {}; inputs = {}

def parts(serial):
    if serial == 0: return 1900, 1, 0
    if serial == 60: return 1900, 2, 29
    d = date(1899, 12, 31) + timedelta(days=serial - (serial > 60))
    return d.year, d.month, d.day

def serial(y, m, d):
    if (y, m, d) == (1900, 2, 29): return 60
    day = date(y, m, 1) + timedelta(days=d-1)
    return (day-date(1899, 12, 31)).days + (day >= date(1900, 3, 1))

for n in range(72):
    row = n+2; y,m,d = parts(n)
    sheet[f'A{row}'] = n; sheet[f'A{row}'].number_format = 'yyyy-mm-dd'
    text = f'{y:04}-{m:02}-{d:02}'
    end = (date(y,m+1,1)-timedelta(days=1)).day
    # EOMONTH uses Gregorian month lengths, unlike DATE's fictional day 60.
    # Firsthand Excel report: https://techcommunity.microsoft.com/discussions/
    # excelgeneral/issue-with-the-sn-for-2291900-when-using-eom-function/4335766
    # Month-shift day zero requires direct Excel qualification; it is not
    # assumed to behave like an ordinary day-of-month in this corpus.
    next_end = (date(y,m+2,1)-timedelta(days=1)).day
    cases = {'B':(f'=DAY(A{row})',d), 'C':(f'=MONTH(A{row})',m),
             'D':(f'=YEAR(A{row})',y), 'E':(f'=WEEKDAY(A{row})',((n+6)%7)+1),
             'F':(f'=DATE(D{row},C{row},B{row})',n),
             'G':(f'=DATEVALUE(TEXT(A{row},"yyyy-mm-dd"))', n if n else '#VALUE!'),
             'H':(f'=TEXT(A{row},"yyyy-mm-dd")',text),
             'J':(f'=EOMONTH(A{row},0)',serial(y,m,end))}
    if n: cases['I']=(f'=EDATE(A{row},1)',serial(y,m+1,min(d,next_end)))
    for col,(formula,value) in cases.items():
        a=f'{col}{row}';sheet[a]=formula;expected[a]=value
    expected[f'A{row}']=n

constructors = [
    ('=DATE(0,1,1)',1), ('=DATE(99,1,1)',serial(1999,1,1)),
    ('=DATE(1899,1,1)',serial(3799,1,1)), ('=DATE(1900,1,0)',0),
    ('=DATE(1900,1,60)',60), ('=DATE(1900,2,29)',60),
    ('=DATE(1900,2,30)',61), ('=DATE(1900,3,0)',60),
    ('=DATE(1900,3,-1)',59), ('=DATE(1900,0,32)',1),
    ('=DATE(1900,13,1)',367), ('=DATE(1901,-10,1)',32),
    ('=DATE(-1,1,1)','#NUM!'), ('=DATE(10000,1,1)','#NUM!'),
    ('=DATE(1900,1,-1)','#NUM!'), ('=DATE(9999,12,31)',2958465),
    ('=DATE(9999,12,32)','#NUM!'), ('=DATE(1900,2147483647,1)','#NUM!'),
    ('=DATE(1900,1,-2147483648)','#NUM!'), ('=DAY(60.75)',29),
    ('=YEAR(1)',1900), ('=MONTH(32)',2), ('=DAY(59)',28),
    ('=VALUE("1900-01-01")',1), ('=VALUE("1900-02-29")',60),
    ('=DATEVALUE("1900-02-28")',59), ('=DATEVALUE("1900-02-29")',60),
    ('=SUM(DAY({0,1,59,60,61}))',59),
    ('=SUM(DATE({0,1900},{1,2},{1,29}))',61),
    ('=WORKDAY(59,1)',60), ('=WORKDAY(60,1)',61),
    ('=WORKDAY(61,-1)',60), ('=WORKDAY(61,-1,{60})',59),
    ('=NETWORKDAYS(59,61)',3), ('=NETWORKDAYS(59,61,{60})',2),
    ('=WORKDAY.INTL(59,1,11)',60), ('=NETWORKDAYS.INTL(59,61,11,{60})',2),
    ('=EDATE(60,12)',serial(1901,2,28)), ('=EDATE(91,-1)',59),
    ('=EDATE(31,1)',59), ('=EDATE(60,0)',59), ('=EDATE(60,1)',89),
    ('=EOMONTH(61,-1)',59), ('=EOMONTH(1,0)',31),
    ('=EOMONTH(60,0)',59), ('=EOMONTH(32,0)',59),
    ('=EOMONTH(DATE(1900,2,1),0)',59),
    ('=EOMONTH(DATE(2000,2,1),0)',serial(2000,2,29)),
    ('=EOMONTH(DATE(2100,2,1),0)',serial(2100,2,28)),
    ('=DAY(-1)','#NUM!'), ('=DAY(2958466)','#NUM!'),
    ('=WEEKDAY(0)',7), ('=WEEKDAY(60,2)',3),
    ('=WORKDAY(2958465,1)','#NUM!'), ('=WORKDAY(0,-1)','#NUM!'),
]
for row,(formula,value) in enumerate(constructors,2):
    a=f'L{row}';sheet[a]=formula;expected[a]=value
for a,value in [('N2','1900-01-01'),('N3','1900-02-28'),('N4','1900-02-29')]:
    # OOXML explicit strings must stay strings; separate native-entry tests
    # submit these same characters to the cell editor as date input.
    sheet[a]=value;inputs[a]=value
for column in 'ABCDEFGHIJKLMN': sheet.column_dimensions[column].width=19
book.save(root/'early-dates.xlsx')
(root/'early-dates.json').write_text(json.dumps({'expected':expected,'explicitText':inputs},indent=2))
print(f'{len(expected)} expectations')
