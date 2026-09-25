"""Paired independent workbook calendars; expectations use Python's Gregorian dates."""
from pathlib import Path
from datetime import datetime, timedelta
import json
from openpyxl import Workbook
from openpyxl.worksheet.formula import ArrayFormula
from openpyxl.utils.datetime import CALENDAR_MAC_1904, CALENDAR_WINDOWS_1900, to_excel

root = Path(__file__).resolve().parents[2] / 'test/fixtures'
dates = [datetime(1904, 1, 1), datetime(1904, 2, 29), datetime(1904, 3, 1),
         datetime(2024, 1, 2), datetime(2024, 2, 29), datetime(9999, 12, 31)]
weekdays = sum((dates[0] + timedelta(days=d)).weekday() < 5 for d in range(61))
six_days = sum((dates[0] + timedelta(days=d)).weekday() != 6 and d != 59 for d in range(61))
outputs = {}
for mac in (False, True):
    w = Workbook(); w.epoch = CALENDAR_MAC_1904 if mac else CALENDAR_WINDOWS_1900
    s = w.active; s.title = 'Calendar'
    s.append(['Date', 'Amount'])
    for i, date in enumerate(dates, 2):
        s.cell(i, 1, date).number_format = 'yyyy-mm-dd'
        s.cell(i, 2, i - 1)
    s['H1'] = 'Date'; s['H2'] = '>=2024-01-01'
    serial = lambda d: to_excel(d, w.epoch)
    maximum = serial(dates[-1])
    cases = [
        ('=YEAR(A2)', 1904), ('=MONTH(A3)', 2), ('=DAY(A3)', 29),
        ('=WEEKDAY(A2,2)', 5), ('=DATE(1904,1,1)', serial(dates[0])),
        ('=DATEVALUE("1904-01-01")', serial(dates[0])),
        ('=VALUE("1904-01-01")', serial(dates[0])),
        ('=TEXT(A2,"yyyy-mm-dd")', '1904-01-01'),
        ('=EDATE(A2,1)', serial(datetime(1904, 2, 1))),
        ('=EOMONTH(A2,1)', serial(dates[1])),
        ('=NETWORKDAYS(A2,A4)', weekdays),
        ('=WORKDAY(A2,1)', serial(datetime(1904, 1, 4))),
        ('=NETWORKDAYS.INTL(A2,A4,11,A3)', six_days),
        ('=WORKDAY.INTL(A2,1,11)', serial(datetime(1904, 1, 2))),
        ('=ISOWEEKNUM(A2)', dates[0].isocalendar().week), ('=WEEKNUM(A2,2)', 1),
        ('=YEARFRAC(A2,A4,3)', 60 / 365), ('=DAYS360(A2,A4)', 60),
        ('=DATEDIF(A2,A4,"d")', 60), ('=SUMIF(A2:A7,">=2024-01-01",B2:B7)', 15),
        ('=COUNTIF(A2:A7,"=1904-01-01")', 1),
        ('=COUNTIFS(A2:A7,">=1904-01-01",A2:A7,"<1904-03-01")', 2),
        ('=INDEX(YEAR(A2:A4),2)', 1904),
        ('=INDEX(TEXT(A2:A3,"yyyy-mm-dd"),2)', '1904-02-29'),
        ('=HOUR(A2+0.5)', 12), ('=TIME(12,0,0)', .5), ('=TIMEVALUE("12:00")', .5),
        ('=MONTH("1904-02-29")', 2), ('=EOMONTH(A7,0)', maximum),
        ('=EDATE(A7,1)', '#NUM!'),
        ('=WORKDAY(A2,-1)', '#NUM!' if mac else serial(datetime(1903,12,31))),
        ('=DAY(-1)', '#NUM!'), (f'=DAY({int(maximum)+1})', '#NUM!'),
        ('=TEXT(A7,"yyyy-mm-dd")', '9999-12-31'),
        ('=TODAY()-DATE(YEAR(TODAY()),MONTH(TODAY()),DAY(TODAY()))', 0),
        ('=DATE(2024,2,29)', serial(dates[4])),
        ('=TEXT("1904-01-01","yyyy-mm-dd")', '1904-01-01'),
        ('=DSUM(A1:B7,"Amount",H1:H2)', 15),
        ('=COUPNCD(A5,DATE(2026,1,2),2,1)', serial(datetime(2024,7,2))),
        ('=COUPPCD(A5,DATE(2026,1,2),2,1)', serial(dates[3])),
        ('=EOMONTH(A7,1)', '#NUM!'), ('=EDATE(A2,2147483647)', '#NUM!'),
        ('=DATE(1904,2147483647,1)', '#NUM!'),
        ('=INDEX(DATE({1904;2024},1,1),1)', serial(dates[0])),
        ('=INDEX(DATE({1904;2024},1,1),2)', serial(datetime(2024,1,1))),
        ('=INDEX(WEEKDAY(A2:A3,2),1)', 5),
        ('=INDEX(ISOWEEKNUM(A2:A3),2)', dates[1].isocalendar().week),
    ]
    expected = {}
    for r, (formula, value) in enumerate(cases, 1):
        a = f'F{r}'
        # A range passed where one value is expected is intersected with the
        # formula's row in a legacy formula (Excel's saved form without
        # dynamic-array metadata). Formulas that compute over a whole range
        # are array formulas, as Excel would save them.
        array = formula.startswith('=INDEX(') and ':' in formula
        s[a] = ArrayFormula(a, formula) if array else formula
        expected[a] = value
        s[f'E{r}'] = formula[1:]
    comparisons = [
        '=PRICE(A5,DATE(2026,1,2),0.05,0.04,100,2,1)',
        '=YIELD(A5,DATE(2026,1,2),0.05,100,100,2,1)',
        '=DURATION(A5,DATE(2026,1,2),0.05,0.04,2,1)',
        '=TBILLPRICE(A5,DATE(2024,7,2),0.03)',
        '=ACCRINT(A2,DATE(1904,7,1),DATE(1904,5,1),0.1,1000,2,1,TRUE)',
        '=AMORLINC(2400,A2,DATE(1904,12,31),300,1,0.15,1)',
        '=XIRR({-1000;1100},A2:A3)',
        '=XNPV(0.1,{-1000;1100},A2:A3)',
        '=COUPDAYS(A5,DATE(2026,1,2),2,1)',
        '=COUPDAYBS(A5,DATE(2026,1,2),2,1)',
        '=COUPDAYSNC(A5,DATE(2026,1,2),2,1)',
        '=COUPNUM(A5,DATE(2026,1,2),2,1)',
        '=DISC(A2,A4,98,100,1)', '=INTRATE(A2,A4,100,102,1)',
        '=PRICEDISC(A2,A4,0.03,100,1)', '=RECEIVED(A2,A4,100,0.03,1)',
        '=YIELDDISC(A2,A4,98,100,1)', '=PRICEMAT(A3,A4,A2,0.05,0.04,1)',
        '=YIELDMAT(A3,A4,A2,0.05,100,1)', '=ACCRINTM(A2,A4,0.05,1000,1)',
        '=MDURATION(A5,DATE(2026,1,2),0.05,0.04,2,1)',
        '=TBILLEQ(A2,A4,0.03)', '=TBILLYIELD(A2,A4,99)',
        '=ODDFPRICE(DATE(2024,2,1),DATE(2026,1,1),DATE(2024,1,1),DATE(2024,7,1),0.05,0.04,100,2,1)',
        '=ODDFYIELD(DATE(2024,2,1),DATE(2026,1,1),DATE(2024,1,1),DATE(2024,7,1),0.05,100,100,2,1)',
        '=ODDLPRICE(DATE(2024,2,1),DATE(2025,3,1),DATE(2024,1,1),0.05,0.04,100,2,1)',
        '=ODDLYIELD(DATE(2024,2,1),DATE(2025,3,1),DATE(2024,1,1),0.05,99,100,2,1)',
        '=AMORDEGRC(2400,A2,DATE(1904,12,31),300,1,0.15,1)',
    ]
    for r, formula in enumerate(comparisons, 1): s[f'J{r}'] = formula
    s['L1'] = '=NOW()-TODAY()'
    s['M1'] = '1904-01-01'; s['M2'] = '25%'; s['M3'] = '$12.50'
    s['D2'].number_format = 'yyyy-mm-dd'
    s.column_dimensions['A'].width = 16
    s.column_dimensions['D'].width = 16
    s.column_dimensions['E'].width = 55
    s.column_dimensions['F'].width = 18
    name = 'date-system-1904' if mac else 'date-system-1900'
    w.save(root / f'{name}.xlsx')
    outputs[name] = {'expected': expected, 'comparisons': [f'J{r}' for r in range(1, len(comparisons)+1)],
                     'dateSerials': {f'A{r}': serial(date) for r, date in enumerate(dates, 2)}}
(root / 'date-systems.json').write_text(json.dumps(outputs, indent=2) + '\n')
print(f'Wrote paired date-system fixtures, {len(cases)} explicit expectations and {len(comparisons)} financial comparisons per workbook.')
