using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;

namespace Ozi.Clock;

class OsClock
{
    public string caption;
    public string timeZone;

    public string bkColor;
    public Grid osGrid;

    private Label lbCapt;
    private Label lbDateMM;
    private Label lbDateDD;
    private Label lbDateDelim;
    private Label lbDateH;
    private Label lbDateM;
    private Label lbDateS;
    //private bool fIsMain;

    public bool isMain
    {
        set
        {
            if (value == true)
            {
                //this.isMain = value;
                lbCapt.FontWeight = FontWeights.DemiBold;
                lbCapt.Foreground = new SolidColorBrush(Colors.White);
            }
            else
            {
                lbCapt.FontWeight = FontWeights.Regular;
                lbCapt.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9"));
            }
        }
    }

    public void setTime(DateTime curTime)
    {
        DateTime tmzTime = TimeZoneInfo.ConvertTimeBySystemTimeZoneId(curTime, timeZone);
        lbDateMM.Content = tmzTime.ToString("MM/");
        lbDateDD.Content = tmzTime.ToString("dd");
        lbDateH.Content = tmzTime.ToString("HH");
        lbDateM.Content = tmzTime.ToString("mm");
        lbDateS.Content = tmzTime.ToString("ss");
    }

    //constructor
    public OsClock(string caption, string timeZone, string bkColor, int position)
    {
        this.caption = caption;
        this.timeZone = timeZone;
        this.bkColor = bkColor;

        Grid newGrid = new Grid();
        newGrid.Width = 99;
        newGrid.Height = 60;
        newGrid.HorizontalAlignment = HorizontalAlignment.Left;
        newGrid.Margin = new Thickness(position, 0, 0, 0);

        LinearGradientBrush br = new LinearGradientBrush();
        br.StartPoint = new Point(0.5, -0.05);
        br.EndPoint = new Point(0.5, 1);
        br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString("#FF383838"),
            0.453)); //"#FF383838"
        br.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString(bkColor), 0.776)); //"#FFAAAAFF"
        newGrid.Background = br;
        newGrid.VerticalAlignment = VerticalAlignment.Bottom;
        osGrid = newGrid;
        this.timeZone = timeZone;

        FontFamily fntClcCaption = new FontFamily("Calibry");

        lbCapt = new Label();
        lbCapt.FontFamily = fntClcCaption;
        lbCapt.FontSize = 16;
        lbCapt.Content = caption + " :";
        lbCapt.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9"));


        newGrid.Children.Add(lbCapt);

        lbDateMM = new Label();
        lbDateMM.FontFamily = fntClcCaption;
        lbDateMM.FontSize = 16;
        lbDateMM.Content = "11/";
        lbDateMM.Foreground = new SolidColorBrush((Color)ColorConverter.ConvertFromString("#FFB9B9B9"));
        lbDateMM.Margin = new Thickness(45, 0, 0, 0);
        newGrid.Children.Add(lbDateMM);

        lbDateDD = new Label();
        lbDateDD.FontFamily = fntClcCaption;
        lbDateDD.FontSize = 16;
        lbDateDD.FontWeight = FontWeights.DemiBold;
        lbDateDD.Content = "11";
        lbDateDD.Foreground = new SolidColorBrush(Colors.White);
        lbDateDD.Margin = new Thickness(70, 0, 0, 0);
        newGrid.Children.Add(lbDateDD);

        lbDateDelim = new Label();
        lbDateDelim.FontFamily = fntClcCaption;
        lbDateDelim.FontSize = 16;
        lbDateDelim.FontWeight = FontWeights.DemiBold;
        lbDateDelim.Content = "       :       :";
        //lbDateDelim.Foreground = new SolidColorBrush(Colors.White);
        lbDateDelim.Margin = new Thickness(0, 35, 0, 0);
        newGrid.Children.Add(lbDateDelim);

        lbDateH = new Label();
        lbDateH.FontFamily = fntClcCaption;
        lbDateH.FontSize = 22;
        lbDateH.FontWeight = FontWeights.DemiBold;
        lbDateH.Content = "00";
        lbDateH.Margin = new Thickness(2, 30, 0, 0);
        newGrid.Children.Add(lbDateH);

        lbDateM = new Label();
        lbDateM.FontFamily = fntClcCaption;
        lbDateM.FontSize = 22;
        lbDateM.Content = "00";
        lbDateM.Margin = new Thickness(38, 30, 0, 0);
        newGrid.Children.Add(lbDateM);

        lbDateS = new Label();
        lbDateS.FontFamily = fntClcCaption;
        lbDateS.FontSize = 16;
        lbDateS.Content = "00";
        lbDateS.Margin = new Thickness(72, 36, 0, 0);
        newGrid.Children.Add(lbDateS);

        //ContextMenu clcMenu = new ContextMenu();

        //MenuItem item1 = new MenuItem();
        //item1.Header = "Exit";
        //clcMenu.Items.Add(item1);

        //newGrid.ContextMenu = clcMenu;
    }
}