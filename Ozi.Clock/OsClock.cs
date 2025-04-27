using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;

namespace Ozi.Utilities;

internal class OsClock
{
    public string Caption;
    private readonly string _timeZone;

    public string BkColor;
    public readonly Grid OsGrid;

    private readonly Label _lbCapt;
    private readonly Label _lbDateMm;
    private readonly Label _lbDateDd;
    private readonly Label _lbDateH;
    private readonly Label _lbDateM;
    private readonly Label _lbDateS;
    //private bool fIsMain;

    public bool IsMain
    {
        set
        {
            if (value == true)
            {
                //this.isMain = value;
                _lbCapt.FontWeight = FontWeights.DemiBold;
                _lbCapt.Foreground = new SolidColorBrush(Colors.White);
            }
            else
            {
                _lbCapt.FontWeight = FontWeights.Regular;
                _lbCapt.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9"));
            }
        }
    }

    public void SetTime(DateTime curTime)
    {
        var tmzTime = TimeZoneInfo.ConvertTimeBySystemTimeZoneId(curTime, _timeZone);
        _lbDateMm.Content = tmzTime.ToString("MM/");
        _lbDateDd.Content = tmzTime.ToString("dd");
        _lbDateH.Content = tmzTime.ToString("HH");
        _lbDateM.Content = tmzTime.ToString("mm");
        _lbDateS.Content = tmzTime.ToString("ss");
    }

    //constructor
    public OsClock(string caption, string timeZone, string bkColor, int position)
    {
        Caption = caption;
        _timeZone = timeZone;
        BkColor = bkColor;

        var newGrid = new Grid
        {
            Width = 99,
            Height = 60,
            HorizontalAlignment = HorizontalAlignment.Left,
            Margin = new Thickness(position, 0, 0, 0)
        };

        var br = new LinearGradientBrush
        {
            StartPoint = new Point(0.5, -0.05),
            EndPoint = new Point(0.5, 1)
        };
        br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString("#FF383838"),
            0.453)); //"#FF383838"
        br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString(bkColor), 0.776)); //"#FFAAAAFF"
        newGrid.Background = br;
        newGrid.VerticalAlignment = VerticalAlignment.Bottom;
        OsGrid = newGrid;
        _timeZone = timeZone;

        var fntClcCaption = new FontFamily("Calibry");

        _lbCapt = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            Content = caption + " :",
            Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9"))
        };


        newGrid.Children.Add(_lbCapt);

        _lbDateMm = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            Content = "11/",
            Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9")),
            Margin = new Thickness(45, 0, 0, 0)
        };
        newGrid.Children.Add(_lbDateMm);

        _lbDateDd = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            FontWeight = FontWeights.DemiBold,
            Content = "11",
            Foreground = new SolidColorBrush(Colors.White),
            Margin = new Thickness(70, 0, 0, 0)
        };
        newGrid.Children.Add(_lbDateDd);

        var lbDateDelim = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            FontWeight = FontWeights.DemiBold,
            Content = "       :       :",
            //lbDateDelim.Foreground = new SolidColorBrush(Colors.White);
            Margin = new Thickness(0, 35, 0, 0)
        };
        newGrid.Children.Add(lbDateDelim);

        _lbDateH = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 22,
            FontWeight = FontWeights.DemiBold,
            Content = "00",
            Margin = new Thickness(2, 30, 0, 0)
        };
        newGrid.Children.Add(_lbDateH);

        _lbDateM = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 22,
            Content = "00",
            Margin = new Thickness(38, 30, 0, 0)
        };
        newGrid.Children.Add(_lbDateM);

        _lbDateS = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            Content = "00",
            Margin = new Thickness(72, 36, 0, 0)
        };
        newGrid.Children.Add(_lbDateS);

        //ContextMenu clcMenu = new ContextMenu();

        //MenuItem item1 = new MenuItem();
        //item1.Header = "Exit";
        //clcMenu.Items.Add(item1);

        //newGrid.ContextMenu = clcMenu;
    }
}