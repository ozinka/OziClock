using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;

namespace Ozi.Utilities;

public class OsClock
{
    private string _caption;
    private string _color;
    public string TimeZoneId;
    public readonly Grid OsGrid;

    public string Caption
    {
        get => _caption;
        set
        {
            _caption = value;
            if (_lbCapt != null)
            {
                _lbCapt.Content = value;
            }
        }
    }
    public string Color
    {
        get => _color;
        set
        {
            _color = value;

            // Only apply brushes if UI elements are already created
            if (OsGrid != null)
            {
                var br = new LinearGradientBrush
                {
                    StartPoint = new Point(0.5, -0.05),
                    EndPoint = new Point(0.5, 1)
                };
                br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString("#FF383838"), 0.453));
                br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString(_color), 0.776));
                OsGrid.Background = br;
            }

            if (RulerGrid?.Children.Count > 0 && RulerGrid.Children[0] is Canvas canvas)
            {
                var linearGradient = new LinearGradientBrush
                {
                    StartPoint = new Point(0.5, 0),
                    EndPoint = new Point(0.5, 1)
                };
                linearGradient.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString(_color),
                    0.959));
                linearGradient.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString("#FF383838"),
                    1.0));

                canvas.Background = linearGradient;
            }
        }
    }
    public Grid RulerGrid { get; set; }

    private readonly Label _lbCapt;
    private readonly Label _lbDateMm;
    private readonly Label _lbDateDd;
    private readonly Label _lbDateH;
    private readonly Label _lbDateM;
    private readonly Label _lbDateS;
    private bool _isMain;

    public bool IsMain
    {
        get => _isMain;
        set
        {
            _isMain = value;

            if (value)
            {
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

    //constructor
    public OsClock(string caption, string timeZoneId, string color, int position, bool IsMain)
    {
        Caption = caption;
        this.TimeZoneId = timeZoneId;
        _color = color;

        OsGrid = new Grid
        {
            Width = 99,
            Height = 60,
            HorizontalAlignment = HorizontalAlignment.Left,
            Margin = new Thickness(0, 0, 1, 0)
        };

        var br = new LinearGradientBrush
        {
            StartPoint = new Point(0.5, -0.05),
            EndPoint = new Point(0.5, 1)
        };
        br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString("#FF383838"), 0.453));
        br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString(_color), 0.776));
        OsGrid.Background = br;
        OsGrid.VerticalAlignment = VerticalAlignment.Bottom;

        this.TimeZoneId = timeZoneId;

        var fntClcCaption = new FontFamily("Calibry");

        _lbCapt = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            Content = caption,
            SnapsToDevicePixels = true,
            Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9"))
        };


        OsGrid.Children.Add(_lbCapt);

        _lbDateMm = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            Content = "11/",
            Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9")),
            Margin = new Thickness(50, 0, 0, 0)
        };
        OsGrid.Children.Add(_lbDateMm);

        _lbDateDd = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            FontWeight = FontWeights.DemiBold,
            Content = "11",
            Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9")),
            Margin = new Thickness(75, 0, 0, 0)
        };
        OsGrid.Children.Add(_lbDateDd);

        var lbDateDelim = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            FontWeight = FontWeights.DemiBold,
            Content = ":",
            //lbDateDelim.Foreground = new SolidColorBrush(Colors.White);
            Margin = new Thickness(29, 35, 0, 0)
        };
        OsGrid.Children.Add(lbDateDelim);

        var lbDateDelim2 = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            FontWeight = FontWeights.DemiBold,
            Content = ":",
            //lbDateDelim.Foreground = new SolidColorBrush(Colors.White);
            Margin = new Thickness(65, 35, 0, 0)
        };
        OsGrid.Children.Add(lbDateDelim2);

        _lbDateH = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 22,
            FontWeight = FontWeights.DemiBold,
            Content = "00",
            Margin = new Thickness(5, 35, 0, 0),
            Padding = new Thickness(0, 0, 0, 2),
            SnapsToDevicePixels = true
        };
        OsGrid.Children.Add(_lbDateH);

        _lbDateM = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 22,
            Content = "00",
            Margin = new Thickness(42, 35, 0, 0),
            Padding = new Thickness(0, 0, 0, 2),
            SnapsToDevicePixels = true
        };
        OsGrid.Children.Add(_lbDateM);

        _lbDateS = new Label
        {
            FontFamily = fntClcCaption,
            FontSize = 16,
            Content = "00",
            Margin = new Thickness(72, 36, 0, 0)
        };
        OsGrid.Children.Add(_lbDateS);

        this.IsMain = IsMain;


        RulerGrid = new Grid
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            Height = 463,
            Margin = new Thickness(0, 0, 1, 0),
            Width = 99
        };
        var canvas = new Canvas
        {
            HorizontalAlignment = HorizontalAlignment.Stretch,
            UseLayoutRounding = true
        };
        var linearGradient = new LinearGradientBrush
        {
            StartPoint = new Point(0.5, 0),
            EndPoint = new Point(0.5, 1)
        };
        linearGradient.GradientStops.Add(
            new GradientStop((Color)ColorConverter.ConvertFromString(_color), 0.959));
        linearGradient.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString("#FF383838"), 1.0));

        canvas.Background = linearGradient;

        Color = _color;

        RulerGrid.Children.Add(canvas);
    }

    public void SetTime(DateTime curTime)
    {
        var tmzTime = TimeZoneInfo.ConvertTimeBySystemTimeZoneId(curTime, TimeZoneId);
        _lbDateMm.Content = tmzTime.ToString("MM'/'");
        _lbDateDd.Content = tmzTime.ToString("dd");
        _lbDateH.Content = tmzTime.ToString("HH");
        _lbDateM.Content = tmzTime.ToString("mm");
        _lbDateS.Content = tmzTime.ToString("ss");
    }
}