# F-150 FORScan spreadsheet export

**Source:** [Google Sheet](https://docs.google.com/spreadsheets/d/1uDSQ1Z5a2Wt8-kjrSiVSlDFGFHnfeuhb3RTMVz95730/edit)  
**Kind:** FORScan **As-Built / configuration** blocks (module `XXX-YY-ZZ` addresses), **not** live Mode `0x22` DID dumps.

**CMFD policy:** display-only — do **not** write As-Built to the truck from mfd.

Use this export for **module identity and feature labels**. Live glass data lives in `live_parameters.csv` + `mfd::obd::ford`.

Sheets: **55** · total data rows ~56090

| # | Sheet | File | Rows |
|---:|---|---|---:|
| 0 | Common | `00_Common.csv` | 1055 |
| 1 | APIM-Old2 | `01_APIM-Old2.csv` | 875 |
| 2 | ABS (2015-2017) | `02_ABS_2015-2017.csv` | 1025 |
| 3 | ABS (2018-2019) | `03_ABS_2018-2019.csv` | 1001 |
| 4 | ACM (2015-17) | `04_ACM_2015-17.csv` | 1126 |
| 5 | ACM 2015-17 | `05_ACM_2015-17.csv` | 1000 |
| 6 | ACM 2018-20 | `06_ACM_2018-20.csv` | 1000 |
| 7 | APIM Sync 1 | `07_APIM_Sync_1.csv` | 1001 |
| 8 | APIM Sync 2 | `08_APIM_Sync_2.csv` | 1001 |
| 9 | APIM Sync 3 | `09_APIM_Sync_3.csv` | 1509 |
| 10 | BCM (JU5T) | `10_BCM_JU5T.csv` | 1000 |
| 11 | ACM (2018+) | `11_ACM_2018.csv` | 1088 |
| 12 | APIM Sync 3 - 2-27-22 | `12_APIM_Sync_3_-_2-27-22.csv` | 992 |
| 13 | BCM (BdyCM)-old | `13_BCM_BdyCM_-old.csv` | 1411 |
| 14 | DDMPDM | `14_DDMPDM.csv` | 1132 |
| 15 | DSM | `15_DSM.csv` | 1001 |
| 16 | DSP (2018+) | `16_DSP_2018.csv` | 1000 |
| 17 | FCDIM | `17_FCDIM.csv` | 998 |
| 18 | FCIM | `18_FCIM.csv` | 1027 |
| 19 | HSWM | `19_HSWM.csv` | 1008 |
| 20 | IPMA (2015-20) | `20_IPMA_2015-20.csv` | 1000 |
| 21 | IPMB | `21_IPMB.csv` | 1000 |
| 22 | IPC | `22_IPC.csv` | 1291 |
| 23 | HSWM - Old | `23_HSWM_-_Old.csv` | 1009 |
| 24 | PAM (2015-20) | `24_PAM_2015-20.csv` | 1000 |
| 25 | APIM-old | `25_APIM-old.csv` | 1061 |
| 26 | IPC-old2 | `26_IPC-old2.csv` | 1084 |
| 27 | IPMA (2015-17) - old | `27_IPMA_2015-17_-_old.csv` | 997 |
| 28 | IPMB - old | `28_IPMB_-_old.csv` | 1000 |
| 29 | IPMB-old | `29_IPMB-old.csv` | 991 |
| 30 | IPC-old | `30_IPC-old.csv` | 1072 |
| 31 | APIM-Backup | `31_APIM-Backup.csv` | 1057 |
| 32 | PAM-old | `32_PAM-old.csv` | 997 |
| 33 | PSCM | `33_PSCM.csv` | 1017 |
| 34 | RCM | `34_RCM.csv` | 1090 |
| 35 | SCCM | `35_SCCM.csv` | 1089 |
| 36 | SCME | `36_SCME.csv` | 1000 |
| 37 | Add NAV to 2015s | `37_Add_NAV_to_2015s.csv` | 1004 |
| 38 | SCMGSCMH | `38_SCMGSCMH.csv` | 912 |
| 39 | SODL SODR | `39_SODL_SODR.csv` | 1000 |
| 40 | SODLSODR-old | `40_SODLSODR-old.csv` | 1007 |
| 41 | TCU-J | `41_TCU-J.csv` | 999 |
| 42 | TCU-K | `42_TCU-K.csv` | 1000 |
| 43 | TRM | `43_TRM.csv` | 65 |
| 44 | TCU-2018 | `44_TCU-2018.csv` | 984 |
| 45 | Add NAV to 2015s - old | `45_Add_NAV_to_2015s_-_old.csv` | 1006 |
| 46 | APIM-New | `46_APIM-New.csv` | 1036 |
| 47 | IPC-Newer | `47_IPC-Newer.csv` | 1035 |
| 48 | Tire Size Values | `48_Tire_Size_Values.csv` | 1022 |
| 49 | Powerpont Timeout | `49_Powerpont_Timeout.csv` | 1008 |
| 50 | Headlights-Draft | `50_Headlights-Draft.csv` | 1007 |
| 51 | BVNI BAPI | `51_BVNI_BAPI.csv` | 1000 |
| 52 | Country Codes | `52_Country_Codes.csv` | 1000 |
| 53 | BVNI BAPI (1) | `53_BVNI_BAPI_1.csv` | 1000 |
| 54 | Country Codes (1) | `54_Country_Codes_1.csv` | 1000 |

Also: `modules_index.csv`, `asbuilt_address_prefixes.csv`, `live_parameters.csv`.
