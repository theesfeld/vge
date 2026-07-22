# F-150 FORScan export — **2019 only**

**Vehicle scope:** 2019 Ford F-150 (P552, e.g. Crew Cab 2.7L XLT).  
**Source sheet:** [Google Sheet](https://docs.google.com/spreadsheets/d/1uDSQ1Z5a2Wt8-kjrSiVSlDFGFHnfeuhb3RTMVz95730/edit)  
**Kind:** FORScan **As-Built / configuration** (module `XXX-YY-ZZ`), **not** live Mode `0x22` PIDs.

**CMFD policy:** display-only — never write As-Built from mfd.

### What was removed

Sheets for **2015–2017 only**, superseded **-old / Backup / Old2**, APIM **Sync 1/2**, “Add NAV to 2015s”, draft headlights, and duplicate Country/BVNI tabs.

### What remains

**32** module sheets applicable to **2019** (plus year bands that include 2019, e.g. 2018–20).

| # | Sheet | File | Rows |
|---:|---|---|---:|
| 0 | ABS 2018-2019 | `00_ABS_2018-2019.csv` | 1005 |
| 1 | ACM 2018-20 | `01_ACM_2018-20.csv` | 1001 |
| 2 | ACM 2018plus | `02_ACM_2018plus.csv` | 1091 |
| 3 | APIM New | `03_APIM_New.csv` | 1036 |
| 4 | APIM Sync 3 | `04_APIM_Sync_3.csv` | 1553 |
| 5 | APIM Sync 3 2022notes | `05_APIM_Sync_3_2022notes.csv` | 994 |
| 6 | BCM JU5T | `06_BCM_JU5T.csv` | 1032 |
| 7 | BVNI BAPI | `07_BVNI_BAPI.csv` | 1000 |
| 8 | Common | `08_Common.csv` | 1065 |
| 9 | Country Codes | `09_Country_Codes.csv` | 1000 |
| 10 | DDMPDM | `10_DDMPDM.csv` | 1132 |
| 11 | DSM | `11_DSM.csv` | 1001 |
| 12 | DSP 2018plus | `12_DSP_2018plus.csv` | 1000 |
| 13 | FCDIM | `13_FCDIM.csv` | 998 |
| 14 | FCIM | `14_FCIM.csv` | 1027 |
| 15 | HSWM | `15_HSWM.csv` | 1008 |
| 16 | IPC | `16_IPC.csv` | 1294 |
| 17 | IPC Newer | `17_IPC_Newer.csv` | 1035 |
| 18 | IPMA 2015-20 | `18_IPMA_2015-20.csv` | 1010 |
| 19 | IPMB | `19_IPMB.csv` | 1001 |
| 20 | PAM 2015-20 | `20_PAM_2015-20.csv` | 1000 |
| 21 | Power Point Timeout | `21_Power_Point_Timeout.csv` | 1008 |
| 22 | PSCM | `22_PSCM.csv` | 1017 |
| 23 | RCM | `23_RCM.csv` | 1090 |
| 24 | SCCM | `24_SCCM.csv` | 1089 |
| 25 | SCME | `25_SCME.csv` | 1000 |
| 26 | SCMGSCMH | `26_SCMGSCMH.csv` | 912 |
| 27 | SODL SODR | `27_SODL_SODR.csv` | 1000 |
| 28 | TCU 2018plus | `28_TCU_2018plus.csv` | 987 |
| 29 | TCU K 2019 | `29_TCU_K_2019.csv` | 1000 |
| 30 | Tire Size Values | `30_Tire_Size_Values.csv` | 1022 |
| 31 | TRM | `31_TRM.csv` | 65 |

Also: `live_parameters.csv` (live glass DIDs), `modules_can.csv`, `modules_index.csv`, `asbuilt_address_prefixes.csv`.
